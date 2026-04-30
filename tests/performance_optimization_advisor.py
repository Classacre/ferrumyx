#!/usr/bin/env python3
"""
Ferrumyx Performance Optimization Advisor
Automated performance analysis and optimization recommendations
"""

import json
import os
import time
from datetime import datetime, timedelta
from typing import Dict, List, Any, Optional, Tuple
from dataclasses import dataclass, asdict
from pathlib import Path
import logging
import argparse
import pandas as pd
import numpy as np
from collections import defaultdict

logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(levelname)s - %(message)s')
logger = logging.getLogger(__name__)

@dataclass
class PerformanceIssue:
    """Represents a performance issue with priority and impact"""
    category: str  # 'critical', 'high', 'medium', 'low'
    component: str  # 'database', 'api', 'memory', 'cpu', 'network', 'scalability'
    title: str
    description: str
    impact_score: float  # 0-10 scale
    confidence: float    # 0-1 scale
    evidence: List[str]
    recommendations: List[str]
    estimated_effort: str  # 'low', 'medium', 'high'
    expected_improvement: str

@dataclass
class OptimizationPlan:
    """Complete optimization plan with prioritized actions"""
    timestamp: datetime
    overall_score: float  # 0-100 performance score

    # Issues by priority
    critical_issues: List[PerformanceIssue]
    high_priority_issues: List[PerformanceIssue]
    medium_priority_issues: List[PerformanceIssue]
    low_priority_issues: List[PerformanceIssue]

    # Quick wins (low effort, high impact)
    quick_wins: List[PerformanceIssue]

    # Implementation phases
    phase_1_immediate: List[PerformanceIssue]  # Fix within 1 week
    phase_2_short_term: List[PerformanceIssue]  # Fix within 1 month
    phase_3_long_term: List[PerformanceIssue]   # Architectural changes

    # Expected outcomes
    projected_improvements: Dict[str, float]
    cost_benefit_analysis: Dict[str, Any]

class PerformanceDataAggregator:
    """Aggregates performance data from multiple sources"""

    def __init__(self, data_dir: str = "."):
        self.data_dir = Path(data_dir)

    def load_regression_test_results(self) -> Optional[Dict[str, Any]]:
        """Load latest performance regression test results"""
        perf_db = self.data_dir / "performance_db"
        if not perf_db.exists():
            return None

        result_files = list(perf_db.glob("regression_test_*.json"))
        if not result_files:
            return None

        latest_file = max(result_files, key=lambda x: x.stat().st_mtime)
        with open(latest_file, 'r') as f:
            return json.load(f)

    def load_database_analysis(self) -> Optional[Dict[str, Any]]:
        """Load latest database performance analysis"""
        db_reports = self.data_dir / "db_performance_reports"
        if not db_reports.exists():
            return None

        report_files = list(db_reports.glob("db_performance_analysis_*.json"))
        if not report_files:
            return None

        latest_file = max(report_files, key=lambda x: x.stat().st_mtime)
        with open(latest_file, 'r') as f:
            return json.load(f)

    def load_scalability_results(self) -> Optional[Dict[str, Any]]:
        """Load latest scalability test results"""
        scale_reports = self.data_dir / "scalability_reports"
        if not scale_reports.exists():
            return None

        report_files = list(scale_reports.glob("scalability_report_*.json"))
        if not report_files:
            return None

        latest_file = max(report_files, key=lambda x: x.stat().st_mtime)
        with open(latest_file, 'r') as f:
            return json.load(f)

    def load_historical_trends(self, days: int = 30) -> Dict[str, pd.DataFrame]:
        """Load historical performance trends"""
        trends = {}

        # Load regression test history
        perf_db = self.data_dir / "performance_db"
        if perf_db.exists():
            benchmark_files = list(perf_db.glob("*.json"))
            if benchmark_files:
                benchmark_data = []
                for file in benchmark_files:
                    try:
                        with open(file, 'r') as f:
                            data = json.load(f)
                            if 'timestamp' in data:
                                benchmark_data.append(data)
                    except:
                        continue

                if benchmark_data:
                    trends['benchmarks'] = pd.DataFrame(benchmark_data)

        return trends

class PerformanceAnalyzer:
    """Analyzes performance data and identifies optimization opportunities"""

    def __init__(self, aggregator: PerformanceDataAggregator):
        self.aggregator = aggregator

    def analyze_regression_issues(self, regression_data: Dict[str, Any]) -> List[PerformanceIssue]:
        """Analyze performance regression test results for issues"""
        issues = []

        if not regression_data or 'results' not in regression_data:
            return issues

        for scenario, data in regression_data['results'].items():
            benchmark = data.get('benchmark_result', {})
            regression = data.get('regression_analysis', {})

            # Check for regressions
            if regression.get('regression_detected', False):
                regressed_metrics = regression.get('regressed_metrics', {})
                for metric, details in regressed_metrics.items():
                    issue = PerformanceIssue(
                        category='high' if 'latency' in metric else 'medium',
                        component='api',
                        title=f"Performance regression in {scenario}: {metric}",
                        description=f"{metric} degraded by {details['deviation']:.2f} "
                                  f"(current: {details['current']:.2f}, baseline: {details['baseline_mean']:.2f})",
                        impact_score=7.0 if 'latency' in metric else 5.0,
                        confidence=regression.get('confidence', 0.95),
                        evidence=[f"Regression detected with {regression.get('confidence', 0.95)*100:.0f}% confidence"],
                        recommendations=[
                            "Investigate recent code changes that may have caused this regression",
                            "Profile the affected endpoint to identify bottlenecks",
                            "Consider rolling back recent changes if performance-critical"
                        ],
                        estimated_effort='medium',
                        expected_improvement=f"Restore {metric} to baseline levels"
                    )
                    issues.append(issue)

            # Check for poor performance regardless of regression
            if benchmark.get('success_rate', 1.0) < 0.95:
                issues.append(PerformanceIssue(
                    category='critical',
                    component='api',
                    title=f"Low success rate in {scenario}",
                    description=f"Success rate: {benchmark.get('success_rate', 0)*100:.1f}% "
                              f"({benchmark.get('successful_requests', 0)}/{benchmark.get('total_requests', 0)} requests)",
                    impact_score=9.0,
                    confidence=1.0,
                    evidence=["Success rate below 95% threshold"],
                    recommendations=[
                        "Check error logs for failure patterns",
                        "Implement retry logic with exponential backoff",
                        "Add circuit breaker pattern for external dependencies"
                    ],
                    estimated_effort='medium',
                    expected_improvement="Improve success rate to >95%"
                ))

            if benchmark.get('p95_latency', 0) > 5000:  # 5 seconds
                issues.append(PerformanceIssue(
                    category='high',
                    component='api',
                    title=f"High latency in {scenario}",
                    description=f"P95 latency: {benchmark.get('p95_latency', 0):.0f}ms",
                    impact_score=6.0,
                    confidence=1.0,
                    evidence=[f"P95 response time exceeds 5000ms threshold"],
                    recommendations=[
                        "Profile the request processing pipeline",
                        "Optimize database queries and add proper indexing",
                        "Implement response caching for frequently requested data"
                    ],
                    estimated_effort='medium',
                    expected_improvement="Reduce P95 latency by 50-70%"
                ))

        return issues

    def analyze_database_issues(self, db_data: Dict[str, Any]) -> List[PerformanceIssue]:
        """Analyze database performance analysis for issues"""
        issues = []

        if not db_data:
            return issues

        plan = db_data.get('optimization_plan', {})
        summary = plan.get('summary', {})
        current_metrics = plan.get('current_metrics', {})

        # Critical database issues
        if summary.get('critical_issues', 0) > 0:
            issues.append(PerformanceIssue(
                category='critical',
                component='database',
                title="Critical database performance issues detected",
                description=f"{summary['critical_issues']} critical database issues require immediate attention",
                impact_score=10.0,
                confidence=1.0,
                evidence=["Database analysis shows critical performance problems"],
                recommendations=[
                    "Review critical database issues immediately",
                    "Consider emergency database maintenance",
                    "Monitor database performance closely"
                ],
                estimated_effort='high',
                expected_improvement="Resolve critical database bottlenecks"
            ))

        # Connection issues
        db_health = current_metrics.get('database_health', {})
        if db_health.get('active_connections', 0) > 50:
            issues.append(PerformanceIssue(
                category='high',
                component='database',
                title="High database connection count",
                description=f"Active connections: {db_health['active_connections']} (threshold: 50)",
                impact_score=7.0,
                confidence=1.0,
                evidence=["Database connection pool under pressure"],
                recommendations=[
                    "Optimize connection pooling configuration",
                    "Implement connection pooling in application code",
                    "Consider database read replicas for read-heavy workloads"
                ],
                estimated_effort='medium',
                expected_improvement="Reduce connection overhead by 30-50%"
            ))

        # Cache issues
        if db_health.get('cache_hit_ratio', 100) < 95:
            issues.append(PerformanceIssue(
                category='medium',
                component='database',
                title="Low database cache hit ratio",
                description=f"Cache hit ratio: {db_health.get('cache_hit_ratio', 0):.1f}%",
                impact_score=5.0,
                confidence=1.0,
                evidence=["Database cache not effectively utilized"],
                recommendations=[
                    "Increase shared_buffers in PostgreSQL configuration",
                    "Optimize frequently accessed data patterns",
                    "Consider query result caching"
                ],
                estimated_effort='medium',
                expected_improvement="Improve cache hit ratio to >95%"
            ))

        # Slow queries
        query_perf = current_metrics.get('query_performance', [])
        slow_queries = [q for q in query_perf if q.get('avg_exec_time_ms', 0) > 1000]
        if slow_queries:
            issues.append(PerformanceIssue(
                category='high',
                component='database',
                title="Slow database queries detected",
                description=f"{len(slow_queries)} queries taking >1000ms average execution time",
                impact_score=8.0,
                confidence=1.0,
                evidence=[f"Query ID {q['query_id']}: {q['avg_exec_time_ms']:.0f}ms" for q in slow_queries[:3]],
                recommendations=[
                    "Analyze and optimize slow queries",
                    "Add appropriate database indexes",
                    "Consider query rewriting for better performance"
                ],
                estimated_effort='medium',
                expected_improvement="Reduce query execution time by 50-80%"
            ))

        return issues

    def analyze_scalability_issues(self, scale_data: Dict[str, Any]) -> List[PerformanceIssue]:
        """Analyze scalability test results for issues"""
        issues = []

        if not scale_data:
            return issues

        efficiency = scale_data.get('scaling_efficiency', 1.0)
        breaking_point = scale_data.get('breaking_point')
        memory_leak = scale_data.get('memory_leak_detected', False)
        resource_trend = scale_data.get('resource_utilization_trend', '')

        # Scaling efficiency issues
        if efficiency < 0.7:
            issues.append(PerformanceIssue(
                category='critical',
                component='scalability',
                title="Poor scaling efficiency",
                description=f"Scaling efficiency: {efficiency:.1%} - system does not scale well with load",
                impact_score=9.0,
                confidence=1.0,
                evidence=["Throughput does not scale linearly with concurrency"],
                recommendations=[
                    "Implement horizontal scaling with load balancer",
                    "Optimize application bottlenecks preventing scaling",
                    "Consider microservices architecture for better scalability"
                ],
                estimated_effort='high',
                expected_improvement="Improve scaling efficiency to >80%"
            ))

        # Breaking point issues
        if breaking_point:
            issues.append(PerformanceIssue(
                category='high',
                component='scalability',
                title=f"Performance breaking point at {breaking_point} users",
                description=f"System performance degrades significantly above {breaking_point} concurrent users",
                impact_score=8.0,
                confidence=1.0,
                evidence=["Significant performance degradation detected"],
                recommendations=[
                    "Implement request queuing for high load periods",
                    "Set up auto-scaling based on this breaking point",
                    "Optimize resource utilization before this threshold"
                ],
                estimated_effort='medium',
                expected_improvement="Handle higher concurrency without performance degradation"
            ))

        # Memory leak issues
        if memory_leak:
            issues.append(PerformanceIssue(
                category='critical',
                component='memory',
                title="Memory leak detected",
                description="Memory usage grows continuously during load testing",
                impact_score=10.0,
                confidence=0.9,
                evidence=["Memory growth detected during scalability testing"],
                recommendations=[
                    "Profile memory allocations and identify leak sources",
                    "Implement proper resource cleanup",
                    "Add memory monitoring and alerts"
                ],
                estimated_effort='high',
                expected_improvement="Eliminate memory leaks and stabilize memory usage"
            ))

        # Resource pressure issues
        if resource_trend == 'high_resource_pressure':
            issues.append(PerformanceIssue(
                category='high',
                component='cpu',
                title="High resource utilization pressure",
                description="System resources under high pressure during scaling tests",
                impact_score=7.0,
                confidence=1.0,
                evidence=["Resource utilization increases significantly with load"],
                recommendations=[
                    "Optimize CPU-intensive operations",
                    "Consider vertical scaling (more powerful instances)",
                    "Implement resource limits and throttling"
                ],
                estimated_effort='medium',
                expected_improvement="Reduce resource utilization pressure by 30-50%"
            ))

        return issues

    def identify_quick_wins(self, all_issues: List[PerformanceIssue]) -> List[PerformanceIssue]:
        """Identify quick wins from all issues"""
        quick_wins = []

        for issue in all_issues:
            # Quick wins are low effort, medium-high impact issues
            if (issue.estimated_effort == 'low' and issue.impact_score >= 5.0) or \
               (issue.estimated_effort == 'medium' and issue.impact_score >= 7.0):
                quick_wins.append(issue)

        # Sort by impact score descending
        quick_wins.sort(key=lambda x: x.impact_score, reverse=True)
        return quick_wins[:5]  # Top 5 quick wins

    def create_implementation_phases(self, all_issues: List[PerformanceIssue]) -> Tuple[List[PerformanceIssue], List[PerformanceIssue], List[PerformanceIssue]]:
        """Categorize issues into implementation phases"""

        # Phase 1: Immediate fixes (critical issues + quick wins)
        phase_1 = [issue for issue in all_issues if issue.category == 'critical']

        # Add high-impact quick wins to phase 1
        quick_wins = self.identify_quick_wins(all_issues)
        for win in quick_wins:
            if win not in phase_1:
                phase_1.append(win)

        # Phase 2: Short-term improvements (high priority issues)
        phase_2 = [issue for issue in all_issues if issue.category == 'high' and issue not in phase_1]

        # Phase 3: Long-term optimizations (medium/low priority, architectural changes)
        phase_3 = [issue for issue in all_issues if issue.category in ['medium', 'low'] and issue not in phase_1 + phase_2]

        return phase_1, phase_2, phase_3

    def calculate_overall_score(self, all_issues: List[PerformanceIssue]) -> float:
        """Calculate overall performance score"""
        base_score = 100.0

        # Deduct points based on issues
        for issue in all_issues:
            if issue.category == 'critical':
                base_score -= issue.impact_score * 2
            elif issue.category == 'high':
                base_score -= issue.impact_score * 1.5
            elif issue.category == 'medium':
                base_score -= issue.impact_score
            else:  # low
                base_score -= issue.impact_score * 0.5

        # Ensure score stays within bounds
        return max(0.0, min(100.0, base_score))

    def project_improvements(self, issues: List[PerformanceIssue]) -> Dict[str, float]:
        """Project performance improvements after implementing fixes"""
        improvements = {
            'response_time_improvement': 0.0,  # percentage
            'throughput_improvement': 0.0,     # percentage
            'resource_efficiency': 0.0,        # percentage
            'reliability_improvement': 0.0     # percentage
        }

        for issue in issues:
            if 'latency' in issue.title.lower() or 'response' in issue.title.lower():
                improvements['response_time_improvement'] += 15.0  # Assume 15% improvement per fix
            if 'throughput' in issue.title.lower() or 'scaling' in issue.title.lower():
                improvements['throughput_improvement'] += 20.0
            if 'memory' in issue.title.lower() or 'cpu' in issue.title.lower():
                improvements['resource_efficiency'] += 10.0
            if 'success' in issue.title.lower() or 'error' in issue.title.lower():
                improvements['reliability_improvement'] += 5.0

        # Cap improvements at reasonable levels
        for key in improvements:
            improvements[key] = min(improvements[key], 80.0)  # Max 80% improvement

        return improvements

class PerformanceOptimizationAdvisor:
    """Main performance optimization advisor"""

    def __init__(self, data_dir: str = "."):
        self.aggregator = PerformanceDataAggregator(data_dir)
        self.analyzer = PerformanceAnalyzer(self.aggregator)

    def generate_optimization_plan(self) -> OptimizationPlan:
        """Generate comprehensive optimization plan"""
        logger.info("Generating performance optimization plan...")

        # Collect all performance data
        regression_data = self.aggregator.load_regression_test_results()
        db_data = self.aggregator.load_database_analysis()
        scale_data = self.aggregator.load_scalability_results()

        # Analyze all issues
        regression_issues = self.analyzer.analyze_regression_issues(regression_data)
        db_issues = self.analyzer.analyze_database_issues(db_data)
        scale_issues = self.analyzer.analyze_scalability_issues(scale_data)

        all_issues = regression_issues + db_issues + scale_issues

        # Categorize issues
        critical_issues = [i for i in all_issues if i.category == 'critical']
        high_issues = [i for i in all_issues if i.category == 'high']
        medium_issues = [i for i in all_issues if i.category == 'medium']
        low_issues = [i for i in all_issues if i.category == 'low']

        # Implementation phases
        phase_1, phase_2, phase_3 = self.analyzer.create_implementation_phases(all_issues)

        # Quick wins
        quick_wins = self.analyzer.identify_quick_wins(all_issues)

        # Calculate scores and projections
        overall_score = self.analyzer.calculate_overall_score(all_issues)
        projected_improvements = self.analyzer.project_improvements(all_issues)

        # Cost-benefit analysis
        total_effort = len([i for i in all_issues if i.estimated_effort == 'high']) * 3 + \
                      len([i for i in all_issues if i.estimated_effort == 'medium']) * 2 + \
                      len([i for i in all_issues if i.estimated_effort == 'low'])

        cost_benefit = {
            'total_issues': len(all_issues),
            'estimated_effort_weeks': total_effort,
            'average_impact_per_issue': sum(i.impact_score for i in all_issues) / len(all_issues) if all_issues else 0,
            'roi_estimate': 'high' if overall_score < 70 else 'medium' if overall_score < 85 else 'low'
        }

        return OptimizationPlan(
            timestamp=datetime.now(),
            overall_score=overall_score,

            critical_issues=critical_issues,
            high_priority_issues=high_issues,
            medium_priority_issues=medium_issues,
            low_priority_issues=low_issues,

            quick_wins=quick_wins,

            phase_1_immediate=phase_1,
            phase_2_short_term=phase_2,
            phase_3_long_term=phase_3,

            projected_improvements=projected_improvements,
            cost_benefit_analysis=cost_benefit
        )

    def generate_report(self, plan: OptimizationPlan) -> str:
        """Generate human-readable optimization report"""
        lines = []
        lines.append("# Ferrumyx Performance Optimization Plan")
        lines.append(f"**Generated:** {plan.timestamp.strftime('%Y-%m-%d %H:%M:%S')}")
        lines.append("")

        # Overall score
        score = plan.overall_score
        grade = "A" if score >= 90 else "B" if score >= 80 else "C" if score >= 70 else "D" if score >= 60 else "F"
        lines.append(f"## Overall Performance Score: {score:.1f}/100 ({grade})")
        lines.append("")

        if score >= 90:
            lines.append("✅ **Excellent performance** - System is performing well with minimal optimization needed.")
        elif score >= 80:
            lines.append("⚠️ **Good performance** - Some optimizations recommended for better efficiency.")
        elif score >= 70:
            lines.append("⚠️ **Fair performance** - Multiple optimizations needed to improve stability.")
        else:
            lines.append("❌ **Poor performance** - Critical issues require immediate attention.")
        lines.append("")

        # Issue summary
        lines.append("## Issue Summary")
        lines.append(f"- **Critical Issues:** {len(plan.critical_issues)}")
        lines.append(f"- **High Priority:** {len(plan.high_priority_issues)}")
        lines.append(f"- **Medium Priority:** {len(plan.medium_priority_issues)}")
        lines.append(f"- **Low Priority:** {len(plan.low_priority_issues)}")
        lines.append("")

        # Quick wins
        if plan.quick_wins:
            lines.append("## 🚀 Quick Wins (High Impact, Low Effort)")
            for i, issue in enumerate(plan.quick_wins, 1):
                lines.append(f"### {i}. {issue.title}")
                lines.append(f"**Impact:** {issue.impact_score:.1f}/10 | **Effort:** {issue.estimated_effort}")
                lines.append(f"**Description:** {issue.description}")
                lines.append("**Recommendations:**")
                for rec in issue.recommendations:
                    lines.append(f"- {rec}")
                lines.append("")

        # Implementation phases
        lines.append("## 📋 Implementation Roadmap")

        if plan.phase_1_immediate:
            lines.append("### Phase 1: Immediate Actions (Fix within 1 week)")
            for i, issue in enumerate(plan.phase_1_immediate, 1):
                lines.append(f"{i}. **{issue.title}** - {issue.component} ({issue.category} priority)")

        if plan.phase_2_short_term:
            lines.append("### Phase 2: Short-term Improvements (Fix within 1 month)")
            for i, issue in enumerate(plan.phase_2_short_term, 1):
                lines.append(f"{i}. **{issue.title}** - {issue.component} ({issue.category} priority)")

        if plan.phase_3_long_term:
            lines.append("### Phase 3: Long-term Optimizations (Architectural changes)")
            for i, issue in enumerate(plan.phase_3_long_term, 1):
                lines.append(f"{i}. **{issue.title}** - {issue.component} ({issue.category} priority)")

        lines.append("")

        # Projected improvements
        lines.append("## 📈 Projected Improvements")
        improvements = plan.projected_improvements
        lines.append(f"- **Response Time:** {improvements['response_time_improvement']:.1f}% improvement")
        lines.append(f"- **Throughput:** {improvements['throughput_improvement']:.1f}% improvement")
        lines.append(f"- **Resource Efficiency:** {improvements['resource_efficiency']:.1f}% improvement")
        lines.append(f"- **Reliability:** {improvements['reliability_improvement']:.1f}% improvement")
        lines.append("")

        # Cost-benefit analysis
        cba = plan.cost_benefit_analysis
        lines.append("## 💰 Cost-Benefit Analysis")
        lines.append(f"- **Total Issues to Address:** {cba['total_issues']}")
        lines.append(f"- **Estimated Effort:** {cba['estimated_effort_weeks']} weeks")
        lines.append(f"- **Average Impact per Issue:** {cba['average_impact_per_issue']:.1f}/10")
        lines.append(f"- **ROI Estimate:** {cba['roi_estimate'].title()}")
        lines.append("")

        # Detailed issue breakdown
        lines.append("## 📋 Detailed Issue Analysis")

        for category, issues, emoji in [
            ("Critical Issues", plan.critical_issues, "🚨"),
            ("High Priority Issues", plan.high_priority_issues, "⚠️"),
            ("Medium Priority Issues", plan.medium_priority_issues, "📋"),
            ("Low Priority Issues", plan.low_priority_issues, "ℹ️")
        ]:
            if issues:
                lines.append(f"### {emoji} {category}")
                for issue in issues:
                    lines.append(f"#### {issue.title}")
                    lines.append(f"**Component:** {issue.component} | **Impact:** {issue.impact_score:.1f}/10 | **Confidence:** {issue.confidence:.1%}")
                    lines.append(f"**Description:** {issue.description}")
                    lines.append("**Evidence:**")
                    for evidence in issue.evidence:
                        lines.append(f"- {evidence}")
                    lines.append("**Recommendations:**")
                    for rec in issue.recommendations:
                        lines.append(f"- {rec}")
                    lines.append("")

        return "\n".join(lines)

    def save_plan(self, plan: OptimizationPlan, output_dir: str = "./optimization_reports"):
        """Save optimization plan and report"""
        output_path = Path(output_dir)
        output_path.mkdir(exist_ok=True)

        timestamp = plan.timestamp.strftime("%Y%m%d_%H%M%S")

        # Save JSON plan
        json_file = output_path / f"optimization_plan_{timestamp}.json"
        with open(json_file, 'w') as f:
            json.dump(asdict(plan), f, indent=2, default=str)

        # Save markdown report
        report = self.generate_report(plan)
        md_file = output_path / f"optimization_plan_{timestamp}.md"
        with open(md_file, 'w') as f:
            f.write(report)

        logger.info(f"Optimization plan saved to: {md_file}")
        return md_file

def main():
    parser = argparse.ArgumentParser(description="Ferrumyx Performance Optimization Advisor")
    parser.add_argument("--data-dir", default=".", help="Directory containing performance data")
    parser.add_argument("--output-dir", default="./optimization_reports", help="Output directory")
    parser.add_argument("--ci-mode", action="store_true", help="CI mode - exit with error code on critical issues")

    args = parser.parse_args()

    # Initialize advisor
    advisor = PerformanceOptimizationAdvisor(args.data_dir)

    try:
        # Generate optimization plan
        plan = advisor.generate_optimization_plan()

        # Save plan and report
        report_file = advisor.save_plan(plan, args.output_dir)

        # Print summary
        print("\n" + "="*60)
        print("FERRUMYX PERFORMANCE OPTIMIZATION ADVISOR")
        print("="*60)
        print(f"Overall Performance Score: {plan.overall_score:.1f}/100")

        issues_summary = {
            'Critical': len(plan.critical_issues),
            'High': len(plan.high_priority_issues),
            'Medium': len(plan.medium_priority_issues),
            'Low': len(plan.low_priority_issues)
        }

        print(f"Issues Found: {sum(issues_summary.values())}")
        for priority, count in issues_summary.items():
            if count > 0:
                print(f"  - {priority}: {count}")

        if plan.quick_wins:
            print(f"\nQuick Wins Available: {len(plan.quick_wins)}")
            for i, win in enumerate(plan.quick_wins[:3], 1):
                print(f"  {i}. {win.title} (Impact: {win.impact_score:.1f})")

        print("\nProjected Improvements:")
        for key, value in plan.projected_improvements.items():
            if value > 0:
                print(f"  - {key.replace('_', ' ').title()}: {value:.1f}%")

        cba = plan.cost_benefit_analysis
        print("\nCost-Benefit Analysis:")
        print(f"  - Estimated Effort: {cba['estimated_effort_weeks']} weeks")
        print(f"  - ROI Estimate: {cba['roi_estimate'].title()}")

        print(f"\nDetailed report: {report_file}")

        # Exit with error code in CI mode if critical issues found
        if args.ci_mode and len(plan.critical_issues) > 0:
            print("\n❌ Critical performance issues detected!")
            exit(1)
        elif plan.overall_score < 70:
            print("\n⚠️ Poor overall performance detected!")
        else:
            print("\n✅ Optimization plan generated successfully")

    except Exception as e:
        logger.error(f"Optimization analysis failed: {e}")
        if args.ci_mode:
            exit(1)
        raise

if __name__ == "__main__":
    main()