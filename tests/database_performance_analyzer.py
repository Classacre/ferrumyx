#!/usr/bin/env python3
"""
Ferrumyx Database Performance Monitoring and Optimization
Query performance monitoring, optimization recommendations, and bottleneck detection
"""

import time
import psycopg2
import psycopg2.extras
import json
import pandas as pd
import numpy as np
from datetime import datetime, timedelta
from typing import Dict, List, Any, Optional, Tuple
from dataclasses import dataclass, asdict
from pathlib import Path
import logging
import argparse
import os
from contextlib import contextmanager

logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(levelname)s - %(message)s')
logger = logging.getLogger(__name__)

@dataclass
class QueryMetric:
    """Represents a database query performance metric"""
    query_id: str
    query_text: str
    execution_time_ms: float
    rows_affected: int
    timestamp: datetime
    connection_count: int
    cache_hit_ratio: float
    temp_files_used: int
    temp_bytes_used: int

@dataclass
class DatabaseHealthMetric:
    """Represents overall database health metrics"""
    timestamp: datetime
    active_connections: int
    total_connections: int
    cache_hit_ratio: float
    dead_tuples_ratio: float
    temp_files_count: int
    temp_files_size_mb: float
    slowest_queries: List[Dict[str, Any]]
    table_bloat_info: List[Dict[str, Any]]

@dataclass
class IndexUsage:
    """Represents index usage statistics"""
    table_name: str
    index_name: str
    scans: int
    tuples_read: int
    tuples_fetched: int
    size_mb: float
    last_used: Optional[datetime]

class DatabaseMonitor:
    """Monitors PostgreSQL database performance"""

    def __init__(self, connection_string: str):
        self.connection_string = connection_string
        self.db_name = self._extract_db_name(connection_string)

    def _extract_db_name(self, conn_str: str) -> str:
        """Extract database name from connection string"""
        # Simple parsing for postgresql://user:pass@host:port/db
        if 'postgresql://' in conn_str:
            return conn_str.split('/')[-1].split('?')[0]
        return 'unknown'

    @contextmanager
    def get_connection(self):
        """Get database connection with proper cleanup"""
        conn = None
        try:
            conn = psycopg2.connect(self.connection_string)
            yield conn
        finally:
            if conn:
                conn.close()

    def get_query_performance(self, limit: int = 20) -> List[QueryMetric]:
        """Get slowest queries from pg_stat_statements"""
        with self.get_connection() as conn:
            cursor = conn.cursor(cursor_factory=psycopg2.extras.RealDictCursor)

            # Enable pg_stat_statements if available
            cursor.execute("""
                SELECT 1 FROM pg_extension WHERE extname = 'pg_stat_statements'
            """)

            if cursor.fetchone():
                # Get query performance from pg_stat_statements
                cursor.execute("""
                    SELECT
                        queryid as query_id,
                        LEFT(query, 200) as query_text,
                        ROUND(total_exec_time / calls, 2) as avg_exec_time_ms,
                        calls as execution_count,
                        ROUND(total_exec_time, 2) as total_exec_time_ms,
                        rows as rows_affected
                    FROM pg_stat_statements
                    WHERE calls > 0
                    ORDER BY avg_exec_time_ms DESC
                    LIMIT %s
                """, (limit,))

                results = cursor.fetchall()

                metrics = []
                for row in results:
                    metrics.append(QueryMetric(
                        query_id=str(row['query_id']),
                        query_text=row['query_text'],
                        execution_time_ms=row['avg_exec_time_ms'],
                        rows_affected=row['rows_affected'],
                        timestamp=datetime.now(),
                        connection_count=0,  # Will be set later
                        cache_hit_ratio=0.0,  # Will be set later
                        temp_files_used=0,
                        temp_bytes_used=0
                    ))

                return metrics
            else:
                logger.warning("pg_stat_statements extension not available")
                return []

    def get_database_health(self) -> DatabaseHealthMetric:
        """Get comprehensive database health metrics"""
        with self.get_connection() as conn:
            cursor = conn.cursor(cursor_factory=psycopg2.extras.RealDictCursor)

            # Get connection counts
            cursor.execute("""
                SELECT
                    COUNT(*) FILTER (WHERE state = 'active') as active_connections,
                    COUNT(*) as total_connections
                FROM pg_stat_activity
                WHERE datname = %s
            """, (self.db_name,))

            conn_stats = cursor.fetchone()

            # Get cache hit ratio
            cursor.execute("""
                SELECT
                    ROUND(
                        100.0 * sum(blks_hit) / (sum(blks_hit) + sum(blks_read)), 2
                    ) as cache_hit_ratio
                FROM pg_stat_database
                WHERE datname = %s
            """, (self.db_name,))

            cache_stats = cursor.fetchone()

            # Get temp file usage
            cursor.execute("""
                SELECT
                    COALESCE(SUM(files), 0) as temp_files_count,
                    ROUND(COALESCE(SUM(bytes), 0) / 1024.0 / 1024.0, 2) as temp_files_size_mb
                FROM pg_stat_database
                WHERE datname = %s
            """, (self.db_name,))

            temp_stats = cursor.fetchone()

            # Get slowest queries (simplified)
            slowest_queries = []
            try:
                cursor.execute("""
                    SELECT
                        LEFT(query, 100) as query_text,
                        ROUND(extract(epoch from (now() - query_start)) * 1000, 2) as runtime_ms
                    FROM pg_stat_activity
                    WHERE state = 'active' AND datname = %s
                    ORDER BY query_start ASC
                    LIMIT 5
                """, (self.db_name,))

                slowest_queries = [dict(row) for row in cursor.fetchall()]
            except Exception as e:
                logger.warning(f"Could not get slowest queries: {e}")

            # Get table bloat information (simplified)
            table_bloat = []
            try:
                cursor.execute("""
                    SELECT
                        schemaname,
                        tablename,
                        ROUND(
                            100.0 * (n_dead_tup::float / (n_live_tup + n_dead_tup + 1)), 2
                        ) as dead_tuple_ratio
                    FROM pg_stat_user_tables
                    WHERE n_live_tup + n_dead_tup > 0
                    ORDER BY dead_tuple_ratio DESC
                    LIMIT 10
                """)

                table_bloat = [dict(row) for row in cursor.fetchall()]
            except Exception as e:
                logger.warning(f"Could not get table bloat info: {e}")

            return DatabaseHealthMetric(
                timestamp=datetime.now(),
                active_connections=conn_stats['active_connections'] or 0,
                total_connections=conn_stats['total_connections'] or 0,
                cache_hit_ratio=cache_stats['cache_hit_ratio'] or 0.0,
                dead_tuples_ratio=0.0,  # Would need more complex calculation
                temp_files_count=temp_stats['temp_files_count'] or 0,
                temp_files_size_mb=temp_stats['temp_files_size_mb'] or 0.0,
                slowest_queries=slowest_queries,
                table_bloat_info=table_bloat
            )

    def get_index_usage(self) -> List[IndexUsage]:
        """Get index usage statistics"""
        with self.get_connection() as conn:
            cursor = conn.cursor(cursor_factory=psycopg2.extras.RealDictCursor)

            cursor.execute("""
                SELECT
                    schemaname,
                    tablename,
                    indexname,
                    idx_scan as scans,
                    idx_tup_read as tuples_read,
                    idx_tup_fetch as tuples_fetched,
                    ROUND(pg_relation_size(indexrelid) / 1024.0 / 1024.0, 2) as size_mb,
                    last_idx_scan as last_used
                FROM pg_stat_user_indexes
                ORDER BY scans DESC, size_mb DESC
            """)

            results = cursor.fetchall()

            return [
                IndexUsage(
                    table_name=f"{row['schemaname']}.{row['tablename']}",
                    index_name=row['indexname'],
                    scans=row['scans'] or 0,
                    tuples_read=row['tuples_read'] or 0,
                    tuples_fetched=row['tuples_fetched'] or 0,
                    size_mb=row['size_mb'] or 0.0,
                    last_used=row['last_used']
                )
                for row in results
            ]

    def get_table_statistics(self) -> List[Dict[str, Any]]:
        """Get table size and access statistics"""
        with self.get_connection() as conn:
            cursor = conn.cursor(cursor_factory=psycopg2.extras.RealDictCursor)

            cursor.execute("""
                SELECT
                    schemaname,
                    tablename,
                    seq_scan as sequential_scans,
                    idx_scan as index_scans,
                    n_tup_ins as inserts,
                    n_tup_upd as updates,
                    n_tup_del as deletes,
                    n_live_tup as live_tuples,
                    n_dead_tup as dead_tuples,
                    ROUND(
                        pg_total_relation_size(schemaname||'.'||tablename) / 1024.0 / 1024.0, 2
                    ) as total_size_mb,
                    ROUND(
                        pg_relation_size(schemaname||'.'||tablename) / 1024.0 / 1024.0, 2
                    ) as table_size_mb,
                    ROUND(
                        pg_total_relation_size(schemaname||'.'||tablename) -
                        pg_relation_size(schemaname||'.'||tablename) / 1024.0 / 1024.0, 2
                    ) as index_size_mb
                FROM pg_stat_user_tables
                ORDER BY total_size_mb DESC
            """)

            return [dict(row) for row in cursor.fetchall()]

    def run_explain_analyze(self, query: str) -> Dict[str, Any]:
        """Run EXPLAIN ANALYZE on a query to get execution plan"""
        with self.get_connection() as conn:
            cursor = conn.cursor()

            try:
                cursor.execute(f"EXPLAIN (ANALYZE, BUFFERS, FORMAT JSON) {query}")
                result = cursor.fetchone()

                if result and result[0]:
                    return {
                        'query': query,
                        'execution_plan': result[0][0] if isinstance(result[0], list) else result[0],
                        'success': True,
                        'error': None
                    }
                else:
                    return {
                        'query': query,
                        'execution_plan': None,
                        'success': False,
                        'error': 'No execution plan returned'
                    }
            except Exception as e:
                return {
                    'query': query,
                    'execution_plan': None,
                    'success': False,
                    'error': str(e)
                }

class DatabaseOptimizer:
    """Provides database optimization recommendations"""

    def __init__(self, monitor: DatabaseMonitor):
        self.monitor = monitor

    def analyze_query_performance(self, metrics: List[QueryMetric]) -> List[str]:
        """Analyze query performance and provide recommendations"""
        recommendations = []

        for metric in metrics:
            if metric.execution_time_ms > 1000:  # Queries taking > 1 second
                recommendations.append(
                    f"Slow query detected ({metric.execution_time_ms:.0f}ms): "
                    f"{metric.query_text[:100]}... - Consider optimization"
                )

            # Check for potential issues
            if 'seq scan' in metric.query_text.lower() and metric.rows_affected > 10000:
                recommendations.append(
                    f"Sequential scan on large table in query: {metric.query_text[:100]}... "
                    "- Consider adding indexes"
                )

            if 'filesort' in metric.query_text.lower():
                recommendations.append(
                    f"Filesort detected in query: {metric.query_text[:100]}... "
                    "- Consider query optimization or index addition"
                )

        return recommendations

    def analyze_health_metrics(self, health: DatabaseHealthMetric) -> List[str]:
        """Analyze database health and provide recommendations"""
        recommendations = []

        # Connection analysis
        if health.active_connections > 50:
            recommendations.append(
                f"High active connections ({health.active_connections}) - "
                "Consider connection pooling optimization"
            )

        # Cache hit ratio analysis
        if health.cache_hit_ratio < 95.0:
            recommendations.append(
                ".1f"
                "Consider increasing shared_buffers or adding more RAM"
            )

        # Temp file analysis
        if health.temp_files_count > 100:
            recommendations.append(
                f"High temp file usage ({health.temp_files_count} files, "
                f"{health.temp_files_size_mb:.1f}MB) - Check work_mem settings"
            )

        # Table bloat analysis
        for table_info in health.table_bloat_info:
            if table_info.get('dead_tuple_ratio', 0) > 20.0:
                recommendations.append(
                    f"High table bloat in {table_info['schemaname']}.{table_info['tablename']} "
                    ".1f"
                    "Consider running VACUUM or REINDEX"
                )

        return recommendations

    def analyze_index_usage(self, indexes: List[IndexUsage]) -> List[str]:
        """Analyze index usage and provide recommendations"""
        recommendations = []

        unused_indexes = []
        large_unused_indexes = []

        for idx in indexes:
            if idx.scans == 0:
                unused_indexes.append(idx)
                if idx.size_mb > 100:  # Large unused indexes
                    large_unused_indexes.append(idx)

        if unused_indexes:
            recommendations.append(
                f"Found {len(unused_indexes)} unused indexes - Consider removal to save space"
            )

        for idx in large_unused_indexes:
            recommendations.append(
                f"Large unused index: {idx.index_name} ({idx.size_mb:.1f}MB) on {idx.table_name} "
                "- Consider removal"
            )

        # Check for missing indexes (look for frequent seq scans)
        table_stats = self.monitor.get_table_statistics()
        for table in table_stats:
            seq_scan_ratio = table['sequential_scans'] / max(table['sequential_scans'] + table['index_scans'], 1)
            if seq_scan_ratio > 0.8 and table['total_size_mb'] > 10:  # Mostly sequential scans on medium-large table
                recommendations.append(
                    f"Table {table['schemaname']}.{table['tablename']} has high sequential scan ratio "
                    ".1f"
                    "Consider adding appropriate indexes"
                )

        return recommendations

    def generate_optimization_plan(self) -> Dict[str, Any]:
        """Generate comprehensive database optimization plan"""
        logger.info("Generating database optimization plan...")

        # Collect all metrics
        query_metrics = self.monitor.get_query_performance()
        health_metrics = self.monitor.get_database_health()
        index_usage = self.monitor.get_index_usage()

        # Analyze and generate recommendations
        query_recommendations = self.analyze_query_performance(query_metrics)
        health_recommendations = self.analyze_health_metrics(health_metrics)
        index_recommendations = self.analyze_index_usage(index_usage)

        # Categorize recommendations by priority
        critical = []
        high = []
        medium = []
        low = []

        all_recommendations = query_recommendations + health_recommendations + index_recommendations

        for rec in all_recommendations:
            if 'critical' in rec.lower() or 'high' in rec.lower():
                high.append(rec)
            elif 'consider' in rec.lower():
                medium.append(rec)
            else:
                low.append(rec)

        # Generate action plan
        action_plan = {
            'timestamp': datetime.now().isoformat(),
            'database': self.monitor.db_name,
            'summary': {
                'total_recommendations': len(all_recommendations),
                'critical_issues': len(critical),
                'high_priority': len(high),
                'medium_priority': len(medium),
                'low_priority': len(low)
            },
            'recommendations': {
                'critical': critical,
                'high': high,
                'medium': medium,
                'low': low
            },
            'current_metrics': {
                'query_performance': [asdict(m) for m in query_metrics[:5]],  # Top 5 slowest
                'database_health': asdict(health_metrics),
                'index_usage': [asdict(idx) for idx in index_usage[:10]]  # Top 10 indexes
            }
        }

        return action_plan

class DatabasePerformanceAnalyzer:
    """Main database performance analysis tool"""

    def __init__(self, connection_string: str, output_dir: str = "./db_performance_reports"):
        self.monitor = DatabaseMonitor(connection_string)
        self.optimizer = DatabaseOptimizer(self.monitor)
        self.output_dir = Path(output_dir)
        self.output_dir.mkdir(exist_ok=True)

    def run_full_analysis(self) -> Dict[str, Any]:
        """Run complete database performance analysis"""
        logger.info("Starting comprehensive database performance analysis...")

        try:
            # Generate optimization plan
            optimization_plan = self.optimizer.generate_optimization_plan()

            # Add additional analysis
            table_stats = self.monitor.get_table_statistics()

            # Generate performance report
            report = {
                'analysis_timestamp': datetime.now().isoformat(),
                'database_name': self.monitor.db_name,
                'optimization_plan': optimization_plan,
                'table_statistics': table_stats,
                'performance_score': self._calculate_performance_score(optimization_plan)
            }

            return report

        except Exception as e:
            logger.error(f"Database analysis failed: {e}")
            return {
                'error': str(e),
                'analysis_timestamp': datetime.now().isoformat(),
                'database_name': self.monitor.db_name
            }

    def _calculate_performance_score(self, optimization_plan: Dict[str, Any]) -> float:
        """Calculate overall database performance score (0-100)"""
        base_score = 100.0

        # Deduct points based on issues
        issues = optimization_plan['summary']

        # Critical issues have highest penalty
        base_score -= issues['critical_issues'] * 20

        # High priority issues
        base_score -= issues['high_priority'] * 10

        # Medium priority issues
        base_score -= issues['medium_priority'] * 5

        # Low priority issues
        base_score -= issues['low_priority'] * 1

        # Health metrics penalties
        health = optimization_plan['current_metrics']['database_health']

        if health['cache_hit_ratio'] < 95:
            base_score -= 10
        if health['active_connections'] > 50:
            base_score -= 5
        if health['temp_files_count'] > 100:
            base_score -= 5

        # Ensure score stays within bounds
        return max(0.0, min(100.0, base_score))

    def generate_report(self, analysis_results: Dict[str, Any]) -> str:
        """Generate human-readable database performance report"""
        report = []
        report.append("# Database Performance Analysis Report")
        report.append(f"**Database:** {analysis_results.get('database_name', 'Unknown')}")
        report.append(f"**Analysis Date:** {analysis_results.get('analysis_timestamp', datetime.now().isoformat())}")
        report.append("")

        if 'error' in analysis_results:
            report.append(f"## Error\n{analysis_results['error']}")
            return "\n".join(report)

        plan = analysis_results['optimization_plan']

        # Performance score
        score = analysis_results.get('performance_score', 0)
        grade = self._get_performance_grade(score)
        report.append(f"## Performance Score: {score:.1f}/100 ({grade})")
        report.append("")

        # Summary
        summary = plan['summary']
        report.append("## Summary")
        report.append(f"- **Total Recommendations:** {summary['total_recommendations']}")
        report.append(f"- **Critical Issues:** {summary['critical_issues']}")
        report.append(f"- **High Priority:** {summary['high_priority']}")
        report.append(f"- **Medium Priority:** {summary['medium_priority']}")
        report.append(f"- **Low Priority:** {summary['low_priority']}")
        report.append("")

        # Current Health Status
        health = plan['current_metrics']['database_health']
        report.append("## Current Database Health")
        report.append(f"- **Active Connections:** {health['active_connections']}")
        report.append(f"- **Total Connections:** {health['total_connections']}")
        report.append(f"- **Cache Hit Ratio:** {health['cache_hit_ratio']:.1f}%")
        report.append(f"- **Temp Files:** {health['temp_files_count']} ({health['temp_files_size_mb']:.1f}MB)")
        report.append("")

        # Recommendations by priority
        recommendations = plan['recommendations']

        if recommendations['critical']:
            report.append("## 🚨 Critical Issues")
            for rec in recommendations['critical']:
                report.append(f"- {rec}")
            report.append("")

        if recommendations['high']:
            report.append("## ⚠️ High Priority Recommendations")
            for rec in recommendations['high']:
                report.append(f"- {rec}")
            report.append("")

        if recommendations['medium']:
            report.append("## 📋 Medium Priority Recommendations")
            for rec in recommendations['medium']:
                report.append(f"- {rec}")
            report.append("")

        if recommendations['low']:
            report.append("## ℹ️ Low Priority Recommendations")
            for rec in recommendations['low']:
                report.append(f"- {rec}")
            report.append("")

        # Top Slow Queries
        if plan['current_metrics']['query_performance']:
            report.append("## Top Slow Queries")
            for i, query in enumerate(plan['current_metrics']['query_performance'][:5], 1):
                report.append(f"### {i}. Query ID: {query['query_id']}")
                report.append(f"- **Avg Execution Time:** {query['execution_time_ms']:.2f}ms")
                report.append(f"- **Execution Count:** {query.get('execution_count', 'N/A')}")
                report.append(f"- **Query:** {query['query_text'][:200]}...")
                report.append("")

        return "\n".join(report)

    def _get_performance_grade(self, score: float) -> str:
        """Convert performance score to letter grade"""
        if score >= 90:
            return "A"
        elif score >= 80:
            return "B"
        elif score >= 70:
            return "C"
        elif score >= 60:
            return "D"
        else:
            return "F"

    def save_report(self, analysis_results: Dict[str, Any], filename: str = None):
        """Save analysis results and report"""
        if filename is None:
            timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
            filename = f"db_performance_analysis_{timestamp}"

        # Save JSON results
        json_file = self.output_dir / f"{filename}.json"
        with open(json_file, 'w') as f:
            json.dump(analysis_results, f, indent=2, default=str)

        # Save markdown report
        report = self.generate_report(analysis_results)
        md_file = self.output_dir / f"{filename}.md"
        with open(md_file, 'w') as f:
            f.write(report)

        logger.info(f"Database performance analysis saved to: {md_file}")
        return md_file

def main():
    parser = argparse.ArgumentParser(description="Ferrumyx Database Performance Analyzer")
    parser.add_argument("--connection-string", default=os.getenv("DATABASE_URL"),
                       help="PostgreSQL connection string")
    parser.add_argument("--output-dir", default="./db_performance_reports",
                       help="Output directory for reports")
    parser.add_argument("--ci-mode", action="store_true",
                       help="CI mode - exit with error code on critical issues")

    args = parser.parse_args()

    if not args.connection_string:
        logger.error("Database connection string required. Set DATABASE_URL or use --connection-string")
        exit(1)

    # Initialize analyzer
    analyzer = DatabasePerformanceAnalyzer(args.connection_string, args.output_dir)

    # Run analysis
    try:
        results = analyzer.run_full_analysis()
        report_file = analyzer.save_report(results)

        print(f"Database performance analysis completed. Report saved to: {report_file}")

        # Print summary
        print("\n" + "="*60)
        print("DATABASE PERFORMANCE ANALYSIS SUMMARY")
        print("="*60)

        if 'error' in results:
            print(f"❌ Analysis failed: {results['error']}")
            exit(1)

        score = results.get('performance_score', 0)
        grade = analyzer._get_performance_grade(score)
        print(f"Performance Score: {score:.1f}/100 ({grade})")

        plan = results['optimization_plan']['summary']
        print(f"Issues Found: {plan['total_recommendations']}")
        print(f"- Critical: {plan['critical_issues']}")
        print(f"- High: {plan['high_priority']}")
        print(f"- Medium: {plan['medium_priority']}")
        print(f"- Low: {plan['low_priority']}")

        # Exit with error in CI mode if critical issues found
        if args.ci_mode and plan['critical_issues'] > 0:
            print("\n❌ Critical database performance issues detected!")
            exit(1)
        elif score < 70:
            print("\n⚠️ Poor database performance detected!")
        else:
            print("\n✅ Database performance analysis completed successfully")

    except Exception as e:
        logger.error(f"Database performance analysis failed: {e}")
        if args.ci_mode:
            exit(1)
        raise

if __name__ == "__main__":
    main()