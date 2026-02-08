# KGGen: Extracting Knowledge Graphs from Plain Text with Language Models

**Authors:** Belinda Mo*, Kyssen Yu*, Joshua Kazdan, Proud Mpala, Lisa Yu, Chris Cundy, Charilaos Kanatsoulis, Sanmi Koyejo

**Affiliations:**
- Stanford University
- University of Toronto
- FAR AI

*Equal Contribution

## Abstract

Recent interest in building foundation models for KGs has highlighted a fundamental challenge: knowledge-graph data is relatively scarce. The best-known KGs are primarily human-labeled, created by pattern-matching, or extracted using early NLP techniques. While human-generated KGs are in short supply, automatically extracted KGs are of questionable quality. 

We present a solution to this data scarcity problem in the form of a text-to-KG generator (KGGen), a package that uses language models to create high-quality graphs from plaintext. Unlike other KG extractors, KGGen clusters related entities to reduce sparsity in extracted KGs. KGGen is available as a Python library (`pip install kg-gen`), making it accessible to everyone. 

Along with KGGen, we release the first benchmark, **Measure of Information in Nodes and Edges (MINE)**, that tests an extractor's ability to produce a useful KG from plain text. We benchmark our new tool against existing extractors and demonstrate far superior performance.

**Code:** https://github.com/stair-lab/kg-gen

---

## Table of Contents

1. [Introduction](#introduction)
2. [Existing Methods](#existing-methods)
3. [KGGen: KGs From Plain Text](#kggen-kgs-from-plain-text)
4. [A Benchmark for Extraction Performance](#a-benchmark-for-extraction-performance)
5. [Results](#results)
6. [Future Work](#future-work)
7. [Related Work](#related-work)
8. [Acknowledgments](#acknowledgments)
9. [Appendices](#appendices)

---

## 1. Introduction

Knowledge graph (KG) applications and Graph Retrieval-Augmented Generation (RAG) systems are increasingly bottlenecked by the scarcity and incompleteness of available KGs. KGs consist of a set of subject-predicate-object triples, and have become a fundamental data structure for information retrieval.

Most real-world KGs, including Wikidata, DBpedia, and YAGO, are far from complete, with many missing relations between entities. The lack of domain-specific and verified graph data poses a serious challenge for downstream tasks such as KG embeddings, graph RAG, and synthetic graph training data.

### The Challenge with Sparse KGs

Embedding algorithms such as TransE rely on abundant relational data to learn high-quality KG representations. TransE represents relationships as vector translations between entity embeddings and has demonstrated strong performance in link prediction when trained on large KGs (e.g., 1M entities and 17M training samples). However, if the KG is sparse or incomplete, embedding models struggle—they cannot learn or infer missing links effectively, degrading performance on knowledge completion and reasoning tasks.

### RAG and Knowledge Graphs

Consider retrieval-augmented generation (RAG) with a language model—this requires a rich external knowledge source to ground its responses. For instance, GraphRAG integrates a KG into the RAG pipeline. In GraphRAG, a language model like GPT-4o is used to extract a KG from a text corpus automatically, and this graph is used for retrieval and reasoning. This structured, graph-based augmentation has been shown to improve multi-hop reasoning and synthesis of information across documents.

However, GraphRAG's performance ultimately depends on the quality of the extracted graph. In practice, automatically constructed graphs can be noisy and incomplete—some false nodes and edges may be introduced and some important ones omitted, which can hinder downstream reasoning.

### Emerging Foundation Models for Graphs

An emerging line of work that builds on graph-based RAG trains neural networks on KG retrieval. For example, GFM-RAG (Graph Foundation Model for RAG) trains a dedicated graph neural network on an extensive collection of KGs, encompassing 60 graphs with over 14 million triples to serve as a foundation model for graph-based retrieval. These efforts underscore the importance of having dense, well-connected KGs to feed into RAG systems.

### Our Contribution

We propose **KGGen** (Text-to-Knowledge-Graph), a package that leverages language models and a clustering algorithm to extract high-quality, dense KGs from text. KGGen addresses knowledge scarcity by enabling the automatic construction of KGs from any textual source rather than being limited to pre-existing databases like Wikipedia.

The package uses an LM-based extractor to read unstructured text and predict subject-predicate-object triples to capture entities and relations. KGGen then applies an iterative LM-based clustering to refine the raw graph. Inspired by crowd-sourcing strategies for entity resolution, the clustering stage has an LM examine the set of extracted nodes and edges to identify which ones refer to the same underlying entities or concepts. Variations in tense, plurality, stemming, or capitalization are normalized in this process—e.g., "labors" might be clustered with "labor" and "New York City" with "NYC."

The resulting KG has far less redundancy and is densely interlinked, making it suitable for downstream use.

### Key Contributions

1. **KGGen Package:** An open-source Python library that uses LMs to extract high-quality KGs from plain text. Available via `pip install kg-gen`.

2. **MINE Benchmark:** The first-ever benchmark for text-to-KG extractors, allowing for a fair comparison of existing methods.

3. **Superior Performance:** KGGen outperforms existing extraction methods by 18% on the MINE benchmark, demonstrating its potential to produce functional KGs using LMs.

---

## 2. Existing Methods

Before describing KGGen, we explain the two leading existing methods for extracting KGs from plain text, which will serve as a basis for comparison throughout this paper.

### 2.1 OpenIE

Open Information Extraction (OpenIE) was implemented by Stanford CoreNLP. It first generates a "dependency parse" for each sentence using the Stanford CoreNLP pipeline. A learned classifier then traverses each edge in the dependency parse, deciding whether to:

- **Yield:** Create a triple
- **Recurse:** Continue processing a clause  
- **Stop:** Stop processing

These decisions split complex sentences into shorter, self-contained clauses. From these clauses, the system produces (subject, relation, object) tuples, each accompanied by a confidence score. Because OpenIE does not require its input text to have a specific structure, it can handle text in any format.

### 2.2 GraphRAG

Microsoft developed GraphRAG, which integrated graph-based knowledge retrieval with language models. As a first step, GraphRAG provides functionality for generating KGs from plain text to use as its database.

In this process:
1. GraphRAG creates a graph by prompting LMs to extract node-entities and relationships between these entities
2. Few-shot prompting provides the LM with examples of "good" extractions
3. GraphRAG aggregates well-connected nodes into "communities" and generates a summary for each community
4. The final graph consists of communities as nodes and summaries of their relationships as edges

---

## 3. KGGen: KGs From Plain Text

Unlike most previous methods of LLM-based KG extraction, KGGen relies on a multi-stage approach involving an LLM (in our case, GPT-4o) to:

1. Extract entities and relations from each source text
2. Aggregate graphs across sources
3. Iteratively cluster entities and relations

We implement these stages in a modular fashion via a new `kg-gen` Python toolkit consisting of:
- A **'generate'** module for extraction
- An **'aggregate'** module for source consolidation
- A **'cluster'** module for dynamic entity resolution

We use the DSPy framework throughout these stages to define signatures that ensure LLM responses are consistent JSON-formatted outputs. We impose strong constraints on the LLM via prompting to reduce the likelihood of semantically dissimilar duplicate entities.

### 3.1 Entity and Relation Extraction ('generate')

The first stage takes unstructured text as input and produces an initial knowledge graph as extracted triples. We invoke the GPT-4o model for each input text through a DSPy signature that instructs the model to output detected entities in a structured format. 

Then, we invoke a second LLM call through DSPy that instructs the model to output the subject-predicate-object relations, given the set of entities and source text. We find this 2-step approach works better to ensure consistency between entities.

### 3.2 Aggregation ('aggregate')

After extracting triples from each source text, we collect all the unique entities and edges across all source graphs and combine them into a single graph. All entities and edges are normalized to be in lowercase letters only. The aggregation step reduces redundancy in the KG. Note that the aggregation step does not require an LLM.

### 3.3 Entity and Edge Clustering ('cluster')

After extraction and aggregation, we typically have a raw graph containing duplicate or synonymous entities and possibly redundant edges. The clustering stage is a key innovation in our KG extraction methodology that aims to merge nodes and edges representing the same real-world entity or concept.

We take an iterative LLM-based approach to clustering, inspired by how a group of humans might gradually agree on consolidating terms. Rather than attempting to solve the entire clustering in one shot (which is intractable for an extensive list of entities), KGGen performs a sequential series of clustering operations for entities:

1. The entire entities list is passed in context to the LLM, and it attempts to extract a single cluster. An optional cluster-instruction string may be passed to decide how to cluster. The default instructions account for close synonyms and differences in tense and plurality.

2. Validate the single cluster using an LLM-as-a-Judge call with a binary response. If it passes, then add the cluster and remove the cluster entities from the entities list.

3. Assign a label to the cluster that most closely captures the shared meaning of entities in the cluster.

4. Repeat steps 1-3 until n loops happen without a successful cluster extraction.

5. Remaining entities are checked batch-by-batch, with batch size b, for whether they should be added to an existing cluster.

6. For each new addition to a cluster, validate the cluster once more using an LLM-as-a-Judge call with a binary response.

7. Repeat steps 5-6 until there are no remaining entities to check.

The same operations are performed on edges, albeit with slightly modified prompts.

#### Example of Clustering Benefit

The clustering process allows us to create dense KGs that admit meaningful embeddings. To give a real example, in one of our raw KGs, we found the entities "vulnerabilities", "vulnerable", and "weaknesses". Although these are different words, they have similar meanings and should be viewed as equivalent in our KG.

---

## 4. A Benchmark for Extraction Performance

Although a handful of existing methods attempt to extract KGs from plain text, it is difficult to measure progress in the field due to the lack of existing benchmarks. To remedy this, we produce the **Measure of Information in Nodes and Edges (MINE)**, the first benchmark that measures a knowledge-graph extractor's ability to capture and distill a body of text into a KG.

### 4.1 MINE Description

MINE involves generating KGs for 100 articles, each representing a distinct source of textual data. Each article is approximately 1,000 words long and is generated by an LLM based on a diverse list of 100 topics that range from history and art to science, ethics, and psychology.

#### Evaluation Process

To evaluate the quality of the generated KGs:

1. **Extract facts:** We extract 15 facts—defined as statements present in the plain text article—from each article by providing an LLM with the article and an extraction prompt.

2. **Manual verification:** We manually verify that the 15 facts are accurate and contained in the article.

3. **Query the KGs:** For each KG generation method, the KG for each article is queried for each of the 15 facts. We do this by determining the top-k nodes most semantically similar to each fact.

4. **Retrieve relevant subgraph:** We determine all the nodes within two relations of one of the top k-nodes. We return all these nodes along with their relations as the result of the query.

5. **LLM evaluation:** The result is evaluated using an LLM with a specific prompt to produce a binary output:
   - **1** if the fact could be inferred from only the information in the queried nodes and relations
   - **0** otherwise

6. **Calculate score:** The final MINE score of each KG generator on a given article is the percentage of 1s across all 15 evaluations.

#### Node Vectorization

The nodes of the resulting KGs are vectorized using the `all-MiniLM-L6-v2` model from SentenceTransformers, enabling cosine similarity assessment of semantic closeness between the short sentence information and the nodes in the graph.

---

## 5. Results

We use MINE to benchmark KGGen against leading existing methods of plain-text-to-KG extraction: OpenIE and GraphRAG.

### 5.1 Evaluations on MINE

**Performance Summary:**
- **KGGen:** 66.07% (average)
- **GraphRAG:** 47.80% (average)
- **OpenIE:** 29.84% (average)

KGGen scored significantly higher, outperforming GraphRAG by ~18% and OpenIE by ~36%.

#### Qualitative Analysis

A key finding is that KGGen consistently generates KGs that are:
- **Dense and coherent:** Capturing critical relationships and information from the articles
- **Concise predicates:** The relation types extracted by KGGen are more concise and generalize more easily than those from GraphRAG and OpenIE

### 5.2 Qualitative Results

#### GraphRAG's Limitations

GraphRAG often generates a minimal number of nodes and connections for an entire article. This sparsity results in the omission of critical relationships and information.

#### OpenIE's Limitations

OpenIE faces several issues:
1. Most nodes are unreasonably long, incoherent phrases
2. Many nodes are redundant copies of one another, adding unnecessary complexity
3. Frequently produces generic nodes such as "it" and "are" that contain no useful information
4. Due to their frequency, these meaningless nodes often end up as some of the most well-connected nodes in the graph

#### KGGen's Advantages

By contrast, KGGen:
- Consistently generates dense and coherent KGs
- Effectively captures critical relationships and information from articles
- Reduces redundancy through its clustering methodology
- Creates meaningful, interpretable node labels

---

## 6. Future Work

Although KGGen beats existing methods by significant margins, the graphs still exhibit problems, like over or under-clustering. More research into better forms of clustering could improve the quality of our KGs. 

Additionally, our benchmark, MINE, currently measures performance on relatively short corpora, whereas KGs are primarily used to handle massive amounts of information efficiently. Future expansions of our benchmark could focus on larger corpora to better measure the practicality of different extraction techniques.

---

## 7. Related Work

Interest in automated methods to produce structured text to store ontologies dates back to at least 2001 when large volumes of plain text began to flood the fledgling internet. KG extraction from unstructured text has seen significant advances through rule-based and LM-powered approaches in the last 15 years.

### Historical Context

**Early work (2000s):**
- YAGO: Used hard-coded rules to develop a KG extracted from Wikipedia containing over five million facts
- Rule-based extraction still has appeal for those producing KGs in multi-modal domains

**Neural approaches (2010s):**
- With the development of modern natural language processing, hard-coded rules generally ceded to more advanced approaches based on neural networks
- **OpenIE:** A two-tiered extraction system using classifiers and natural logic inference
- **Stanford KBP:** An early approach to using deep networks for entity extraction

### Transformer-based Approaches

As early as 2015, some hypothesized that extracting KGs would go hand-in-hand with developing better language models. More recently, evidence has emerged that transformer-based architectures can identify complex relationships between entities, leading to a wave of transformer-based KG extraction techniques, ranging from fully automatic to human-assisted.

### Our Contribution to the Field

Our contribution to the extraction literature is to build KGs conducive to embedding algorithms such as TransE and TransR. We observed that when one extracts KGs from plaintext, the nodes and relations are often so specific that they are unique. This causes the estimation of embeddings to be under-specified. We develop a method for automatic KG extraction from plain text that clusters similar nodes and edges to prevent this under-specification, leading to a KG with better connectivity and more functional nodes and edges.

### KG Evaluation Methods

**Early approaches:** Focused primarily on directly assessing aspects such as completeness and connectivity or using rule-based statistical methods

**Recent approaches:** Emphasize usability in downstream applications and incorporation of semantic coherence

**Notable methods:**
- **LP-Measure:** Assesses KG quality through link prediction tasks
- **KGTtm (Triple Trustworthiness Measurement):** Evaluates the coherence of triples within a knowledge graph
- **KGrEaT:** Provides comprehensive assessment by evaluating KG performance on downstream tasks
- **DiffQ:** Uses embedding models to evaluate KG quality

### Task-based Evaluation Paradigm

The shift towards task-based evaluation underscores the importance of usability and accessibility in KGs. Factors such as expressiveness, context information, and ease of integration into downstream AI applications are now central to evaluating their quality and effectiveness.

---

## 8. Acknowledgments

JK acknowledges support from NSF grant number DGE-1656518. SK acknowledges support from NSF 2046795 and 2205329, the MacArthur Foundation, Stanford HAI, and Google Inc.

---

## Appendices

### Appendix A: Prompts for KG Extraction

#### Entity Extraction Prompt

```
Extract key entities from the given text. Extracted entities are nouns, verbs, or adjectives, 
particularly regarding sentiment. This is for an extraction task, please be thorough and 
accurate to the reference text.
```

#### Relation Extraction Prompt

```
Extract subject-predicate-object triples from the assistant message. A predicate (1-3 words) 
defines the relationship between the subject and object. Relationship may be fact or sentiment 
based on assistant's message. Subject and object are entities. Entities provided are from the 
assistant message and prior conversation history, though you may not need all of them. This is 
for an extraction task, please be thorough, accurate, and faithful to the reference text.
```

#### Entity Clustering Prompt

```
Find ONE cluster of related entities from this list.

A cluster should contain entities that are the same in meaning, with different:
- tenses
- plural forms
- stem forms
- upper/lower cases

Or entities with close semantic meanings.

Return only if you find entities that clearly belong together.

If you can't find a clear cluster, return an empty list.
```

#### Node Cluster Validation Prompt

```
Verify if these entities belong in the same cluster.

A cluster should contain entities that are the same in meaning, with different:
- tenses
- plural forms
- stem forms
- upper/lower cases

Or entities with close semantic meanings.

Return the entities that you are confident belong together as a single cluster.

If you're not confident, return an empty list.
```

#### Edge Clustering Prompt

```
Find ONE cluster of closely related predicates from this list.

A cluster should contain predicates that are the same in meaning, with different:
- tenses
- plural forms
- stem forms
- upper/lower cases

Predicates are the relations between subject and object entities. Ensure that the predicates 
in the same cluster have very close semantic meanings to describe the relation between the 
same subject and object entities.

Return only if you find predicates that clearly belong together.

If you can't find a clear cluster, return an empty list.
```

#### Edge Cluster Validation Prompt

```
Verify if these predicates belong in the same cluster.

A cluster should contain predicates that are the same in meaning, with different:
- tenses
- plural forms
- stem forms
- upper/lower cases

Predicates are the relations between subject and object entities. Ensure that the predicates 
in the same cluster have very close semantic meanings to describe the relation between the 
same subject and object entities.

Return the predicates that you are confident belong together as a single cluster.

If you're not confident, return an empty list.
```

### Appendix B: Validation of KG Extraction

This section uses the same prompts as Appendix A for validating the KG extraction method.

### Appendix C: Prompts for MINE

#### Fact Extraction Prompt

```
Extract 15 basic, single pieces of information from the following text that describe how 
one object relates to another. Present the pieces of info in short sentences and DO NOT 
include info not directly present in the text. Your output should be of the form 
["info1", "info2" ,..., "info15"]. Make sure the strings are valid Python strings.
```

#### Fact Evaluation Prompt

```
ROLE: You are an evaluator that checks if the correct answer can be deduced from the 
information in the context.

TASK: Determine whether the context contains the information stated in the correct answer.

Respond with "1" if yes, and "0" if no. Do not provide any explanation, just the number.
```

### Appendix D: Example Article from MINE

**Title:** The Rise of Cryptocurrencies

**Content:** 

Cryptocurrencies have taken the financial world by storm in recent years, revolutionizing the way we think about money and transactions. From the creation of Bitcoin in 2009 by an anonymous individual or group known as Satoshi Nakamoto, to the thousands of altcoins that have since emerged, cryptocurrencies have become a significant player in the global economy.

One of the key factors contributing to the rise of cryptocurrencies is the decentralized nature of these digital assets. Unlike traditional fiat currencies that are controlled by governments and central banks, cryptocurrencies operate on a peer-to-peer network, allowing for transactions to occur directly between users without the need for intermediaries. This decentralization not only provides users with more control over their funds but also enhances security and privacy.

Another driving force behind the popularity of cryptocurrencies is the technology that underpins them – blockchain. Blockchain is a distributed ledger technology that ensures the transparency and immutability of transactions on the network. Each transaction is recorded in a block and linked to the previous block, forming a chain of blocks that cannot be altered once validated by the network. This technology has been instrumental in building trust and confidence in cryptocurrencies, as it eliminates the need for a trusted third party to oversee transactions.

The concept of decentralization and blockchain technology has also paved the way for various applications beyond just digital currencies. Smart contracts, for example, are self-executing contracts with the terms of the agreement directly written into code. These contracts automatically enforce and execute themselves when predefined conditions are met, eliminating the need for intermediaries and streamlining processes in various industries.

Cryptocurrencies have also gained traction due to their potential for financial inclusion. In many parts of the world, traditional banking services are inaccessible or too costly for a significant portion of the population. Cryptocurrencies offer a way for individuals to access financial services, such as transferring money and making payments, without the need for a traditional bank account. This has the potential to empower individuals in underserved communities and drive economic growth.

The volatile nature of cryptocurrencies has attracted both investors seeking high returns and speculators looking to capitalize on price fluctuations. The rapid appreciation of certain cryptocurrencies, such as Bitcoin, has led to a surge in interest from retail and institutional investors alike. While this volatility presents opportunities for profit, it also poses risks, as prices can fluctuate dramatically in a short period.

Regulation has been a contentious issue in the cryptocurrency space, with governments and regulatory bodies grappling with how to oversee this emerging asset class. Some countries have embraced cryptocurrencies and blockchain technology, recognizing their potential for innovation and economic growth. Others have taken a more cautious approach, citing concerns about money laundering, tax evasion, and consumer protection.

Despite the challenges and uncertainties surrounding cryptocurrencies, their rise has been undeniable. As more individuals and businesses adopt digital currencies for transactions and investments, the landscape of finance is evolving rapidly. The future of cryptocurrencies remains uncertain, but their impact on the financial world is already profound.

In conclusion, the rise of cryptocurrencies can be attributed to their decentralized nature, blockchain technology, financial inclusion potential, investment opportunities, and regulatory challenges. As these digital assets continue to gain acceptance and adoption, they are reshaping the way we think about money and finance. Whether cryptocurrencies will become mainstream or remain on the fringes of the financial system remains to be seen, but their impact is undeniable and will likely continue to unfold in the years to come.

---

## References

Key citations in the paper include foundational work on KGs, RAG systems, and extraction methods. For a complete reference list, please consult the original arXiv paper.

---

**Document Generated:** February 14, 2025

**Original Source:** https://arxiv.org/html/2502.09956v1
