# Ferrumyx User Guide

## Getting Started

Ferrumyx is an autonomous oncology discovery system that helps researchers identify and prioritize therapeutic targets through literature-driven analysis. This guide will walk you through your first interactions with the system.

### Prerequisites

Before you begin, ensure you have:
- Access to a Ferrumyx instance (local installation or hosted service)
- Basic familiarity with oncology research concepts
- A WhatsApp, Slack, Discord, or web interface account (depending on your setup)

### Initial Setup

#### 1. Access the System

**Web Interface:**
- Navigate to your Ferrumyx web URL (e.g., `http://localhost:3000`)
- No authentication required for basic access

**Chat Interfaces:**
- WhatsApp: Message the Ferrumyx number provided by your administrator
- Slack/Discord: Join the configured workspace and find the Ferrumyx bot
- Type `/help` to see available commands

#### 2. Verify System Health

```bash
# Check if the system is running
curl http://your-ferrumyx-url/health
```

Expected response:
```json
{
  "status": "healthy",
  "version": "2.0.0",
  "services": ["database", "agent", "web"]
}
```

## Basic Usage

### Your First Query

Let's start with a simple oncology research query:

**Via Web Interface:**
1. Navigate to the Query page
2. Enter: "Find KRAS targets in pancreatic cancer"
3. Click "Search"

**Via Chat:**
```
User: Find KRAS targets in pancreatic cancer
Ferrumyx: I'll search for KRAS-related targets in pancreatic cancer literature...

[Analysis in progress...]

Found 3 high-priority targets:
1. KRAS G12D mutation (Score: 9.2/10)
   - Evidence: 45 papers, strong genetic correlation
   - Suggested next steps: Structure analysis, ligand screening

2. EGFR amplification (Score: 8.7/10)
   - Evidence: 32 papers, downstream signaling
   - Clinical trials: 3 active Phase II studies

3. CDK4/6 pathway (Score: 8.1/10)
   - Evidence: 28 papers, cell cycle regulation
   - Drug targets: Palbociclib, Ribociclib
```

### Understanding Results

Ferrumyx returns structured results with:
- **Target Priority Score**: 0-10 scale based on evidence strength
- **Evidence Count**: Number of supporting papers
- **Key Findings**: Summarized research insights
- **Next Steps**: Suggested follow-up analyses

### Common Query Patterns

#### Target Discovery
```
"Find therapeutic targets for BRCA1 mutated breast cancer"
"Identify druggable targets in lung adenocarcinoma"
"Search for novel targets in glioblastoma"
```

#### Literature Analysis
```
"Summarize recent KRAS inhibitor trials"
"What are the resistance mechanisms to EGFR inhibitors?"
"Review combination therapies for melanoma"
```

#### Molecular Analysis
```
"Analyze protein structure of EGFR kinase domain"
"Find binding sites for KRAS G12C inhibitors"
"Predict drug interactions for BRAF V600E"
```

## Advanced Features

### Conversational Workflows

Ferrumyx supports multi-turn conversations for complex analyses:

```
User: I'm studying KRAS in pancreatic cancer
Ferrumyx: I understand you're researching KRAS in pancreatic cancer. What specific aspect interests you?

User: Focus on G12D mutation and potential inhibitors
Ferrumyx: Focusing on KRAS G12D mutations. Let me analyze the literature...

[Deep analysis of G12D-specific evidence...]

User: Can you check for clinical trials?
Ferrumyx: Searching clinicaltrials.gov for KRAS G12D pancreatic cancer trials...

Found 5 relevant trials:
1. NCT04380753 - Phase I, AMG 510 (sotorasib)
2. NCT03600883 - Phase II, MRTX849 (adagrasib)
...
```

### Custom Analysis Pipelines

#### Structure Analysis
```
User: Analyze the structure of KRAS protein and identify druggable pockets
Ferrumyx: I'll perform structural analysis of KRAS...

- Retrieved PDB structure 4OBE
- Identified 3 druggable pockets
- Generated binding site visualizations
- Predicted ligand affinities for known inhibitors
```

#### Literature Mining
```
User: Perform comprehensive literature mining for TP53 mutations in leukemia
Ferrumyx: Starting comprehensive literature analysis...

- Searched 50,000+ papers
- Extracted 1,247 mutation-function relationships
- Built evidence network with 892 entities
- Ranked targets by therapeutic potential
```

### Multi-Channel Collaboration

Ferrumyx supports seamless transitions between channels:

1. **Start research on web interface**
2. **Continue discussion via WhatsApp**
3. **Share results in Slack team channel**
4. **Get notifications via Discord**

### Automated Monitoring

Set up automated literature monitoring:

```
User: Monitor new publications about immunotherapy in renal cell carcinoma
Ferrumyx: Setting up automated monitoring...

- Created monitoring profile: "RCC Immunotherapy"
- Frequency: Daily searches
- Sources: PubMed, bioRxiv, clinicaltrials.gov
- Notifications: Slack channel #research-updates
```

## Research Workflows

### Drug Discovery Pipeline

1. **Target Identification**
   ```
   "Find novel targets in triple-negative breast cancer"
   ```

2. **Evidence Evaluation**
   ```
   "Assess the evidence strength for BRCA1 as a target in TNBC"
   ```

3. **Literature Review**
   ```
   "Summarize clinical trials targeting BRCA1 mutations"
   ```

4. **Molecular Analysis**
   ```
   "Analyze BRCA1 protein structure and identify binding sites"
   ```

5. **Compound Screening**
   ```
   "Find FDA-approved drugs that might target BRCA1 pathway"
   ```

### Biomarker Discovery

1. **Biomarker Identification**
   ```
   "Find prognostic biomarkers in colorectal cancer"
   ```

2. **Validation Analysis**
   ```
   "Validate CEA as a biomarker in CRC using meta-analysis"
   ```

3. **Clinical Correlation**
   ```
   "Correlate biomarker expression with survival in TCGA CRC data"
   ```

### Resistance Mechanism Analysis

1. **Resistance Pattern Identification**
   ```
   "Analyze resistance mechanisms to EGFR inhibitors in NSCLC"
   ```

2. **Alternative Target Discovery**
   ```
   "Find bypass pathways when EGFR is inhibited"
   ```

3. **Combination Strategy Design**
   ```
   "Suggest combination therapies to overcome EGFR resistance"
   ```

## Data Export and Integration

### Export Options

Ferrumyx supports multiple export formats:

- **JSON**: Complete analysis results with metadata
- **CSV**: Tabular data for statistical analysis
- **PDF**: Formatted reports for publications
- **API**: Programmatic access for custom integrations

### API Integration

```python
import requests

# Query Ferrumyx programmatically
response = requests.post('http://localhost:3000/api/chat', json={
    'message': 'Find KRAS targets in pancreatic cancer',
    'thread_id': 'research-123'
})

results = response.json()
# Process results...
```

### Third-Party Integrations

Ferrumyx integrates with:
- **Electronic Lab Notebooks**: Export results to ELN systems
- **Project Management**: Sync findings with Jira/Trello
- **Data Warehouses**: Export to Snowflake/Redshift
- **Visualization Tools**: Connect to Tableau/PowerBI

## Best Practices

### Query Optimization

1. **Be Specific**: Include cancer type, gene names, mutation details
2. **Use Medical Terminology**: Ferrumyx understands oncology-specific language
3. **Iterate**: Start broad, then narrow down based on results
4. **Combine Modalities**: Use both literature and molecular analysis

### Result Interpretation

1. **Check Evidence Strength**: Higher scores indicate stronger evidence
2. **Review Source Quality**: Prefer recent, high-impact journals
3. **Validate Clinically**: Cross-reference with clinical trial data
4. **Consider Context**: Account for tumor heterogeneity and patient populations

### Collaboration

1. **Share Thread IDs**: Use consistent thread IDs for related research
2. **Document Methods**: Keep records of analysis parameters
3. **Version Control**: Track changes in research hypotheses
4. **Peer Review**: Share findings with colleagues for validation

## Troubleshooting

### Common Issues

**No Results Found**
- Try broader search terms
- Check spelling of gene names
- Verify cancer type terminology

**Slow Response Times**
- Complex queries may take several minutes
- Monitor progress via web interface
- Consider breaking large queries into smaller ones

**Inconsistent Results**
- Clear conversation context and restart
- Use specific thread IDs for consistency
- Report issues to system administrators

### Getting Help

- **In-System Help**: Type `/help` in any chat interface
- **Documentation**: Access full documentation at `/docs`
- **Community**: Join the Ferrumyx user community
- **Support**: Contact your system administrator

## Next Steps

Now that you're familiar with basic usage, explore:

1. **Advanced Queries**: Learn complex analysis techniques
2. **API Integration**: Build custom applications
3. **Team Collaboration**: Set up shared research workspaces
4. **Custom Workflows**: Develop specialized analysis pipelines

For more detailed information, see the [Developer Guide](../developer/developer-guide.md) and [API Reference](../developer/api-reference.md).</content>
<parameter name="filePath">D:\AI\Ferrumyx\USER_GUIDE.md