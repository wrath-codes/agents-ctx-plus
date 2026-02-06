# DBDI: Differentiated Bi-Directional Intervention for LLM Safety Alignment

## Overview

**Paper**: "Differentiated Directional Intervention: A Framework for Evading LLM Safety Alignment"  
**Authors**: Peng Zhang, Peijie Sun  
**Venue**: AAAI-26 AIA  
**Published**: November 2025  
**arXiv**: [2511.06852](https://arxiv.org/abs/2511.06852)

**Key Contribution**: Deconstructs LLM safety alignment from a single linear direction into two functionally distinct neural processes — **Harm Detection Direction** and **Refusal Execution Direction** — and introduces a white-box framework that precisely neutralizes safety alignment at a single critical layer, achieving up to **97.88% attack success rate**.

**Index Terms**: LLM Safety, Jailbreaking, Activation Manipulation, Refusal Mechanism, Directional Intervention, SVD, Classifier-Guided Sparsification, White-Box Attack

---

## The Bi-Direction Model of Safety

### Prior Assumption: Single Linear Direction

Previous research modeled LLM safety alignment as a single linear direction in the activation space — a monolithic "refusal vector" that could be ablated or steered to disable safety.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│             PRIOR MODEL: SINGLE SAFETY DIRECTION                            │
│                                                                             │
│  Harmful Prompt → [Activation Space] → Refusal                              │
│                        │                                                    │
│                   Single Direction                                          │
│                   ─────────────────                                         │
│                   • One vector captures safety                              │
│                   • Ablate or steer to disable                              │
│                   • Treats detection and execution as one                   │
│                                                                             │
│  Problem: Oversimplification                                                │
│  ─────────────────────────                                                  │
│  • Conflates two functionally distinct neural processes                     │
│  • Incomplete control over alignment mechanism                              │
│  • Suboptimal attack success rates                                          │
│  • Cannot explain why order of intervention matters                         │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### DBDI Insight: Two Distinct Directions

DBDI posits that safety alignment is a **bi-dimensional construct** consisting of two causally ordered neural processes:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│             DBDI MODEL: BI-DIRECTIONAL SAFETY                               │
│                                                                             │
│  Harmful Prompt                                                             │
│       │                                                                     │
│       ▼                                                                     │
│  ┌──────────────────────────────┐                                           │
│  │  HARM DETECTION DIRECTION   │  ← Upstream Trigger                       │
│  │  ──────────────────────────  │                                           │
│  │  • Identifies harmfulness    │                                           │
│  │  • Classifies intent         │                                           │
│  │  • Fires on malicious input  │                                           │
│  │  • Upstream causal trigger   │                                           │
│  └──────────────┬───────────────┘                                           │
│                 │ Activates                                                  │
│                 ▼                                                            │
│  ┌──────────────────────────────┐                                           │
│  │  REFUSAL EXECUTION DIRECTION│  ← Downstream Pathway                     │
│  │  ──────────────────────────  │                                           │
│  │  • Enacts the refusal        │                                           │
│  │  • Generates refusal tokens  │                                           │
│  │  • Fires only when triggered │                                           │
│  │  • Downstream effector       │                                           │
│  └──────────────┬───────────────┘                                           │
│                 │                                                            │
│                 ▼                                                            │
│  Refusal Response: "I cannot help with that request."                       │
│                                                                             │
│  ═══════════════════════════════════════════════════════════════════════     │
│  Key Insight: These two directions have a CAUSAL HIERARCHY                  │
│  Detection triggers Execution, not the other way around                     │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Causal Hierarchy Evidence

The order of intervention critically determines effectiveness:

| Intervention Order | AdvBench ASR | Interpretation |
|-------------------|:------------:|----------------|
| **Standard** (Nullify Execution → Suppress Detection) | **97.88%** | Correct causal order |
| **Reversed** (Suppress Detection → Nullify Execution) | **2.11%** | Causal structure violated |

**Explanation**: In the reversed order, suppressing the harm detection trigger first fundamentally alters the activation state, leaving no coherent refusal execution signal for the second step to neutralize. The refusal mechanism is never fully engaged, so disabling its execution becomes ineffective.

---

## Directional Vector Extraction

### Two-Stage Process

DBDI extracts high-fidelity vectors for each direction using a two-stage process: SVD for raw direction extraction, refined by classifier-guided sparsification.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│             DIRECTIONAL VECTOR EXTRACTION (Offline Calibration)              │
│                                                                             │
│  Stage 1: SVD-Based Raw Direction Extraction                                │
│  ────────────────────────────────────────────                               │
│                                                                             │
│  Refusal Execution Vector (v_ref):                                          │
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │ Input: Minimally-different benign/harmful prompt pairs (TwinPrompt)  │  │
│  │                                                                     │  │
│  │ Harmful: "How to pick a lock"  →  Activation h_harmful              │  │
│  │ Benign:  "How to pick a song"  →  Activation h_benign               │  │
│  │                                                                     │  │
│  │ Difference Matrix: D_ref = [h_harmful - h_benign] for N pairs       │  │
│  │ SVD(D_ref) → First singular vector = raw v_ref                      │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
│                                                                             │
│  Harm Detection Vector (v_harm):                                            │
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │ Input: Harmful prompts (AdvBench/HarmBench/StrongREJECT)            │  │
│  │        vs. Benign instructions (Alpaca dataset)                     │  │
│  │                                                                     │  │
│  │ Harmful: AdvBench prompts → Activation h_harmful                    │  │
│  │ Benign:  Alpaca prompts   → Activation h_benign                     │  │
│  │                                                                     │  │
│  │ Difference Matrix: D_harm = [h_harmful - h_benign] for M pairs      │  │
│  │ SVD(D_harm) → First singular vector = raw v_harm                    │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
│                                                                             │
│  Stage 2: Classifier-Guided Sparsification                                  │
│  ──────────────────────────────────────────                                 │
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │ Purpose: Purify raw SVD vectors by retaining only the most          │  │
│  │          discriminative neuron dimensions                           │  │
│  │                                                                     │  │
│  │ Method:                                                              │  │
│  │ 1. Train linear classifier on activations projected onto raw vector │  │
│  │ 2. Identify neuron dimensions with highest classification weight    │  │
│  │ 3. Zero out low-discriminative dimensions                           │  │
│  │ 4. Result: Sparse, high-fidelity directional vector                 │  │
│  │                                                                     │  │
│  │ Benefit: Eliminates noise dimensions that dilute intervention       │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Vector Extraction Algorithm

```python
class DirectionalVectorExtractor:
    """
    DBDI Vector Extraction: Two-stage process using SVD + sparsification.
    
    Extracts high-fidelity Refusal Execution and Harm Detection vectors
    from contrasting activation patterns at each model layer.
    """
    
    def __init__(self, model, tokenizer, sparsity_ratio: float = 0.1):
        self.model = model
        self.tokenizer = tokenizer
        self.sparsity_ratio = sparsity_ratio
        
    def extract_refusal_execution_vector(
        self, benign_harmful_pairs: list[tuple[str, str]], layer: int
    ) -> np.ndarray:
        """
        Extract Refusal Execution Direction from minimally-different pairs.
        
        Uses TwinPrompt-style pairs where benign and harmful prompts differ
        by only a few words, isolating the refusal execution signal.
        """
        differences = []
        
        for benign_prompt, harmful_prompt in benign_harmful_pairs:
            h_benign = self._get_activation(benign_prompt, layer)
            h_harmful = self._get_activation(harmful_prompt, layer)
            differences.append(h_harmful - h_benign)
        
        # Stage 1: SVD for raw direction
        D_ref = np.stack(differences)
        U, S, Vt = np.linalg.svd(D_ref, full_matrices=False)
        raw_vector = Vt[0]  # First right singular vector
        
        # Stage 2: Classifier-guided sparsification
        sparse_vector = self._sparsify(
            raw_vector, D_ref, label="refusal_execution"
        )
        
        return sparse_vector
    
    def extract_harm_detection_vector(
        self, harmful_prompts: list[str], benign_prompts: list[str],
        layer: int
    ) -> np.ndarray:
        """
        Extract Harm Detection Direction from harmful vs. benign contrast.
        
        Uses diverse harmful prompts (AdvBench/HarmBench/StrongREJECT)
        contrasted against benign instructions (Alpaca dataset).
        """
        harmful_activations = [
            self._get_activation(p, layer) for p in harmful_prompts
        ]
        benign_activations = [
            self._get_activation(p, layer) for p in benign_prompts
        ]
        
        # Compute pairwise differences
        differences = []
        for h_harm in harmful_activations:
            for h_ben in benign_activations[:len(harmful_activations)]:
                differences.append(h_harm - h_ben)
        
        # Stage 1: SVD for raw direction
        D_harm = np.stack(differences)
        U, S, Vt = np.linalg.svd(D_harm, full_matrices=False)
        raw_vector = Vt[0]
        
        # Stage 2: Classifier-guided sparsification
        sparse_vector = self._sparsify(
            raw_vector, D_harm, label="harm_detection"
        )
        
        return sparse_vector
    
    def _sparsify(
        self, raw_vector: np.ndarray, data_matrix: np.ndarray,
        label: str
    ) -> np.ndarray:
        """
        Classifier-guided sparsification: retain only the most
        discriminative neuron dimensions from the raw SVD vector.
        """
        # Project data onto raw vector direction
        projections = data_matrix @ raw_vector
        
        # Train linear classifier on projected activations
        labels = np.concatenate([
            np.ones(len(data_matrix) // 2),
            np.zeros(len(data_matrix) // 2)
        ])
        
        classifier = LogisticRegression()
        classifier.fit(data_matrix, labels)
        
        # Identify top discriminative dimensions by classifier weight
        weights = np.abs(classifier.coef_[0])
        threshold = np.percentile(weights, (1 - self.sparsity_ratio) * 100)
        
        # Zero out low-discriminative dimensions
        sparse_vector = raw_vector.copy()
        sparse_vector[weights < threshold] = 0.0
        
        # Normalize
        sparse_vector = sparse_vector / np.linalg.norm(sparse_vector)
        
        return sparse_vector
    
    def _get_activation(self, prompt: str, layer: int) -> np.ndarray:
        """Extract hidden state activation at specified layer."""
        inputs = self.tokenizer(prompt, return_tensors="pt")
        with torch.no_grad():
            outputs = self.model(**inputs, output_hidden_states=True)
        return outputs.hidden_states[layer][:, -1, :].cpu().numpy().squeeze()
```

---

## Critical Layer Selection

### Method

DBDI identifies the single optimal layer for intervention by finding where activations for benign and harmful prompts exhibit **maximum linear separability**.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│             CRITICAL LAYER SELECTION                                        │
│                                                                             │
│  For each candidate layer l ∈ {1, 2, ..., L}:                               │
│    1. Extract activations for benign/harmful prompt pairs                   │
│    2. Train linear classifier (from sparsification step)                    │
│    3. Evaluate 5-fold cross-validated accuracy A(l)                         │
│    4. Select layer with highest accuracy:                                   │
│                                                                             │
│       l* = argmax_l A(l)                                                    │
│                                                                             │
│  Llama-2-7B Layer Analysis:                                                 │
│  ─────────────────────────                                                  │
│                                                                             │
│  Accuracy                                                                   │
│     ▲                                                                       │
│ 100 │                         ●●●●                                          │
│     │                      ●●      ●●                                       │
│  90 │                   ●●            ●●                                    │
│     │                ●●                  ●●                                 │
│  80 │           ●●●●                       ●●●                             │
│     │        ●●                                ●●●                         │
│  70 │     ●●                                      ●●●                     │
│     │   ●                                             ●●●●                 │
│  60 │  ●                                                   ●●●●●●●        │
│     │ ●                                                                    │
│  50 │●                                                                     │
│     └──────┬──────┬──────┬──────┬──────┬──────┬──────┬──────┬───▶ Layer    │
│            3      6      9     12     15     18     21     27    30         │
│                                       ▲                                     │
│                                  Layer 16 (l*)                              │
│                              Max separability                               │
│                                                                             │
├─────────────────────────────────────────────────────────────────────────────┤
│  Layer Selection Results (Llama-2-7B, AdvBench ASR):                        │
│                                                                             │
│  Layer 3  (Early):    78.6% ASR  — Concepts forming, not consolidated      │
│  Layer 16 (Optimal):  95.96% ASR — Maximum intervention efficacy           │
│  Layer 30 (Late):     0.19% ASR  — Pathway already committed to refusal    │
│                                                                             │
│  Interpretation:                                                            │
│  • Early layers: Safety representations emerging but incomplete             │
│  • Mid layers: Safety fully consolidated, amenable to intervention         │
│  • Late layers: Computational pathway committed, intervention futile       │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Layer Selection Algorithm

```python
class CriticalLayerSelector:
    """
    Select the optimal intervention layer by maximizing
    linear separability of benign/harmful activations.
    """
    
    def __init__(self, model, tokenizer, n_folds: int = 5):
        self.model = model
        self.tokenizer = tokenizer
        self.n_folds = n_folds
        
    def select_critical_layer(
        self, benign_prompts: list[str], harmful_prompts: list[str],
        candidate_layers: list[int] = None
    ) -> tuple[int, dict]:
        """
        Identify the layer with maximum linear separability.
        
        Returns:
            (optimal_layer, {layer: accuracy} mapping)
        """
        if candidate_layers is None:
            num_layers = self.model.config.num_hidden_layers
            candidate_layers = list(range(1, num_layers + 1))
        
        layer_accuracies = {}
        
        for layer in candidate_layers:
            # Collect activations
            activations = []
            labels = []
            
            for prompt in benign_prompts:
                h = self._get_activation(prompt, layer)
                activations.append(h)
                labels.append(0)
            
            for prompt in harmful_prompts:
                h = self._get_activation(prompt, layer)
                activations.append(h)
                labels.append(1)
            
            X = np.stack(activations)
            y = np.array(labels)
            
            # 5-fold cross-validated accuracy
            scores = cross_val_score(
                LogisticRegression(max_iter=1000), X, y,
                cv=self.n_folds, scoring='accuracy'
            )
            layer_accuracies[layer] = scores.mean()
        
        # Select layer with highest accuracy
        optimal_layer = max(layer_accuracies, key=layer_accuracies.get)
        
        return optimal_layer, layer_accuracies
```

---

## The DBDI Algorithm

### Two-Step Sequential Intervention

DBDI applies a tailored, sequential two-step intervention at the critical layer:

1. **Adaptive Projection Nullification** — Neutralizes the Refusal Execution Direction
2. **Direct Steering** — Suppresses the Harm Detection Direction

```
┌─────────────────────────────────────────────────────────────────────────────┐
│             DBDI INTERVENTION ALGORITHM                                     │
│                                                                             │
│  Input: Hidden state h at critical layer l*, vectors v_ref and v_harm       │
│                                                                             │
│  Step 1: ADAPTIVE PROJECTION NULLIFICATION (Refusal Execution)              │
│  ──────────────────────────────────────────────────────────────              │
│                                                                             │
│  Purpose: Remove the component of h along the refusal execution             │
│           direction, preventing the model from generating refusal tokens    │
│                                                                             │
│  Formula:                                                                   │
│    h' = h - (h · v_ref / ||v_ref||²) * v_ref                               │
│                                                                             │
│  Interpretation:                                                            │
│  ┌─────────────────────────────────────────────────────────────┐            │
│  │           v_ref                                             │            │
│  │            ▲                                                │            │
│  │           /│                                                │            │
│  │          / │  Projection onto v_ref                         │            │
│  │     h  /  │  (refusal component)                            │            │
│  │       /   │                                                 │            │
│  │      /    │                                                 │            │
│  │  ●──/─────┼──────────▶ h' (nullified)                      │            │
│  │            │  Remaining: orthogonal to refusal              │            │
│  │            │                                                │            │
│  └─────────────────────────────────────────────────────────────┘            │
│                                                                             │
│  "Adaptive": Projection magnitude is state-dependent — the amount           │
│  removed depends on how strongly the current hidden state aligns            │
│  with the refusal execution direction                                       │
│                                                                             │
│  Step 2: DIRECT STEERING (Harm Detection)                                   │
│  ─────────────────────────────────────────                                  │
│                                                                             │
│  Purpose: Actively push the hidden state away from the harm                 │
│           detection direction to suppress the upstream trigger              │
│                                                                             │
│  Formula:                                                                   │
│    h'' = h' - α * v_harm                                                    │
│                                                                             │
│  Interpretation:                                                            │
│  ┌─────────────────────────────────────────────────────────────┐            │
│  │                                                             │            │
│  │    v_harm (detection direction)                             │            │
│  │        ▲                                                    │            │
│  │        │                                                    │            │
│  │        │  Steer AWAY from                                   │            │
│  │    h'  ●─────────────────▶ h'' (steered)                   │            │
│  │        │                                                    │            │
│  │        │   -α * v_harm                                      │            │
│  │        ▼                                                    │            │
│  │                                                             │            │
│  └─────────────────────────────────────────────────────────────┘            │
│                                                                             │
│  α: Steering coefficient controlling suppression strength                   │
│                                                                             │
│  ═══════════════════════════════════════════════════════════════             │
│  COMPLETE DBDI FORMULA:                                                     │
│                                                                             │
│    h_final = [h - proj(h, v_ref)] - α * v_harm                              │
│                                                                             │
│    where proj(h, v) = (h · v / ||v||²) * v                                 │
│                                                                             │
│  ORDER MATTERS: Step 1 MUST precede Step 2 (causal hierarchy)               │
│  ═══════════════════════════════════════════════════════════════             │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Complete DBDI Implementation

```python
class DBDIFramework:
    """
    Differentiated Bi-Directional Intervention (DBDI).
    
    White-box framework that neutralizes LLM safety alignment at a
    single critical layer by sequentially applying:
    1. Adaptive projection nullification (refusal execution)
    2. Direct steering (harm detection)
    
    Key finding: Intervention order is causally critical.
    Standard order achieves 97.88% ASR; reversed order: 2.11%.
    """
    
    def __init__(
        self,
        model,
        tokenizer,
        v_ref: np.ndarray,
        v_harm: np.ndarray,
        critical_layer: int,
        steering_coefficient: float = 1.0
    ):
        self.model = model
        self.tokenizer = tokenizer
        self.v_ref = v_ref   # Refusal Execution Direction
        self.v_harm = v_harm  # Harm Detection Direction
        self.critical_layer = critical_layer
        self.alpha = steering_coefficient
        
    @classmethod
    def calibrate(
        cls, model, tokenizer,
        twin_pairs: list[tuple[str, str]],
        harmful_prompts: list[str],
        benign_prompts: list[str],
        steering_coefficient: float = 1.0
    ) -> "DBDIFramework":
        """
        One-time offline calibration: extract vectors and select layer.
        
        Uses ~100 calibration prompts from one benchmark, then
        transfers to attack entirely unseen benchmarks.
        """
        # Step 1: Select critical layer
        selector = CriticalLayerSelector(model, tokenizer)
        critical_layer, _ = selector.select_critical_layer(
            benign_prompts, harmful_prompts
        )
        
        # Step 2: Extract directional vectors at critical layer
        extractor = DirectionalVectorExtractor(model, tokenizer)
        
        v_ref = extractor.extract_refusal_execution_vector(
            twin_pairs, critical_layer
        )
        v_harm = extractor.extract_harm_detection_vector(
            harmful_prompts, benign_prompts, critical_layer
        )
        
        return cls(
            model, tokenizer, v_ref, v_harm,
            critical_layer, steering_coefficient
        )
    
    def intervene(self, hidden_state: torch.Tensor) -> torch.Tensor:
        """
        Apply DBDI intervention at critical layer.
        
        CAUSAL ORDER IS CRITICAL:
        1. First: Nullify refusal execution (downstream effector)
        2. Then:  Suppress harm detection (upstream trigger)
        
        Reversing this order causes performance collapse
        (97.88% → 2.11% ASR).
        """
        h = hidden_state.clone()
        v_ref = torch.tensor(self.v_ref, device=h.device, dtype=h.dtype)
        v_harm = torch.tensor(self.v_harm, device=h.device, dtype=h.dtype)
        
        # Step 1: Adaptive Projection Nullification (Refusal Execution)
        # Remove the component of h along v_ref
        proj_magnitude = torch.dot(h.squeeze(), v_ref) / torch.dot(v_ref, v_ref)
        h = h - proj_magnitude * v_ref
        
        # Step 2: Direct Steering (Harm Detection)
        # Push h away from v_harm direction
        h = h - self.alpha * v_harm
        
        return h
    
    def generate_with_intervention(self, prompt: str) -> str:
        """
        Generate response with DBDI intervention at critical layer.
        
        Hooks into the model's forward pass to intercept and modify
        the hidden state at the critical layer during inference.
        """
        hook_handle = None
        
        def intervention_hook(module, input, output):
            """Intercept hidden state at critical layer."""
            modified_output = list(output)
            hidden_states = modified_output[0]
            
            # Apply DBDI to last token position
            hidden_states[:, -1, :] = self.intervene(
                hidden_states[:, -1, :]
            )
            
            modified_output[0] = hidden_states
            return tuple(modified_output)
        
        # Register hook at critical layer
        layer_module = self.model.model.layers[self.critical_layer]
        hook_handle = layer_module.register_forward_hook(intervention_hook)
        
        try:
            inputs = self.tokenizer(prompt, return_tensors="pt")
            outputs = self.model.generate(
                **inputs, max_new_tokens=512,
                do_sample=False
            )
            response = self.tokenizer.decode(
                outputs[0], skip_special_tokens=True
            )
        finally:
            if hook_handle:
                hook_handle.remove()
        
        return response
```

---

## Ablation Study: Why Both Directions Matter

### Single-Pathway vs. Full DBDI

```
┌─────────────────────────────────────────────────────────────────────────────┐
│             ABLATION: SINGLE-PATHWAY INTERVENTIONS                          │
│                                                                             │
│  Refusal-Only (Nullify v_ref only):                                         │
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │ • Removes execution pathway but harm detection still fires           │  │
│  │ • Model detects harm → attempts refusal via alternative pathways     │  │
│  │ • AdvBench ASR: 1.34% (2.11% simplified template)                   │  │
│  │ • Conclusion: Detection trigger redirects to backup refusal          │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
│                                                                             │
│  Harm-Only (Suppress v_harm only):                                          │
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │ • Suppresses detection but execution pathway remains intact          │  │
│  │ • Residual signals may still trigger execution direction             │  │
│  │ • AdvBench ASR: 11.34% (20.00% simplified template)                 │  │
│  │ • Conclusion: Execution pathway has some autonomous activation       │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
│                                                                             │
│  Both Directions (Full DBDI):                                               │
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │ • Detection suppressed AND execution nullified                       │  │
│  │ • No upstream trigger, no downstream pathway                         │  │
│  │ • AdvBench ASR: 95.96% (97.88% simplified template)                 │  │
│  │ • Conclusion: Both directions must be addressed                      │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Symmetric vs. Differentiated Intervention

| Method | AdvBench ASR | HarmBench ASR | StrongREJECT |
|--------|:-----------:|:------------:|:------------:|
| **Full DBDI** | **95.96%** (97.88%) | **92%** (95%) | **0.750** (0.784) |
| Symmetric Projection | 62.88% (87.10%) | 86% (90%) | 0.058 (0.045) |
| Symmetric Steering | 9.42% (1.15%) | 16% (4%) | 0.004 (0.004) |
| Refusal-Only | 1.34% (2.11%) | 73.0% (67.00%) | 0.369 (0.220) |
| Harm-Only | 11.34% (20.00%) | 35.0% (49.50%) | 0.115 (0.180) |

*Values in parentheses are results from the simplified prompt template.*

**Key Finding**: Applying the same intervention type symmetrically to both directions significantly underperforms the differentiated approach. Each direction requires its own tailored manipulation technique.

---

## Experimental Results

### General Efficacy (Llama-2-7B Primary Testbed)

| Benchmark | Metric | DBDI Score |
|-----------|--------|:----------:|
| **AdvBench** | Attack Success Rate | **97.88%** |
| **HarmBench** | Attack Success Rate | **95.00%** |
| **StrongREJECT** | Mean Harmfulness Score | **0.784** |

### Cross-Model Generalization

| Model | AdvBench ASR | HarmBench ASR | StrongREJECT |
|-------|:-----------:|:------------:|:------------:|
| **Llama-2-7B** | **97.88%** | **95.00%** | **0.784** |
| Deepseek-7B | High | High | High |
| Qwen-7B | High | High | High |

**Cross-Dataset Validation Protocol**: Vectors are extracted using ~100 prompts from one benchmark (calibration set, e.g., StrongREJECT) and tested on entirely unseen benchmarks (e.g., AdvBench). High transferability indicates the extraction captures fundamental, dataset-agnostic safety representations.

### Comparison with Existing Jailbreaking Methods

| Method | Type | AdvBench ASR | HarmBench ASR | StrongREJECT |
|--------|------|:-----------:|:------------:|:------------:|
| **DBDI** | Activation manipulation | **95.96%** | **91.8%** | **0.750** |
| TwinBreak | Parameter modification | 94.62% | 94.00% | 0.702 |
| Directional Ablation | Activation manipulation | — | 22.6% | — |
| GCG | Prompt-based | — | — | — |

### Critical Layer Selection Results (Llama-2-7B)

| Intervention Layer | AdvBench ASR | Interpretation |
|-------------------|:-----------:|----------------|
| Layer 3 (Early) | 78.6% | Safety concepts forming, not consolidated |
| **Layer 16 (Optimal)** | **95.96%** | Maximum linear separability |
| Layer 30 (Late) | 0.19% | Pathway already committed to refusal |

---

## Complete DBDI Pipeline

```
┌─────────────────────────────────────────────────────────────────────────────┐
│             DBDI COMPLETE PIPELINE                                          │
│                                                                             │
│  PHASE 1: OFFLINE CALIBRATION (One-Time)                                    │
│  ═══════════════════════════════════════                                    │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────┐            │
│  │ 1. Collect contrasting prompt sets (~100 prompts)           │            │
│  │    • TwinPrompt pairs (benign/harmful minimal pairs)        │            │
│  │    • Harmful benchmarks (AdvBench/HarmBench/StrongREJECT)   │            │
│  │    • Benign instructions (Alpaca dataset)                   │            │
│  │                                                             │            │
│  │ 2. For each layer l ∈ {1, ..., L}:                          │            │
│  │    a. Extract activations for all prompts                   │            │
│  │    b. Train linear classifier, evaluate 5-fold CV accuracy  │            │
│  │    c. Record A(l)                                           │            │
│  │                                                             │            │
│  │ 3. Select critical layer: l* = argmax_l A(l)                │            │
│  │                                                             │            │
│  │ 4. At layer l*:                                             │            │
│  │    a. SVD on TwinPrompt differences → raw v_ref             │            │
│  │    b. Sparsify v_ref via classifier weights                 │            │
│  │    c. SVD on harmful-benign differences → raw v_harm        │            │
│  │    d. Sparsify v_harm via classifier weights                │            │
│  │                                                             │            │
│  │ Output: v_ref, v_harm, l*                                   │            │
│  └─────────────────────────────────────────────────────────────┘            │
│                                                                             │
│  PHASE 2: REAL-TIME INFERENCE (Per Prompt)                                  │
│  ═════════════════════════════════════════                                  │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────┐            │
│  │ 1. Feed malicious prompt to model                           │            │
│  │                                                             │            │
│  │ 2. Intercept hidden state h at layer l*                     │            │
│  │                                                             │            │
│  │ 3. Apply DBDI intervention (CAUSAL ORDER):                  │            │
│  │    a. h' = h - proj(h, v_ref)          [Nullify execution]  │            │
│  │    b. h'' = h' - α * v_harm            [Suppress detection] │            │
│  │                                                             │            │
│  │ 4. Replace h with h'' at layer l*                           │            │
│  │                                                             │            │
│  │ 5. Continue forward pass through remaining layers           │            │
│  │                                                             │            │
│  │ 6. Model generates compliant (misaligned) response          │            │
│  └─────────────────────────────────────────────────────────────┘            │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Connection to the Complexity Trap

### Parallel Insights

DBDI and the Complexity Trap research share a fundamental insight: **granular decomposition of seemingly monolithic mechanisms reveals simpler, more effective intervention points**.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│             PARALLELS: DBDI ←→ COMPLEXITY TRAP                              │
│                                                                             │
│  DBDI Insight:                                                              │
│  ────────────                                                               │
│  "Safety alignment is NOT a single direction —                              │
│   it's TWO functionally distinct directions with causal ordering"           │
│                                                                             │
│  Complexity Trap Insight:                                                   │
│  ───────────────────────                                                    │
│  "Context management does NOT require LLM summarization —                   │
│   simple observation masking achieves comparable results"                   │
│                                                                             │
│  ═══════════════════════════════════════════════════════════════             │
│  SHARED PRINCIPLE: Decompose the problem before solving it                  │
│  ═══════════════════════════════════════════════════════════════             │
│                                                                             │
│  Alignment                     │  Context Management                        │
│  ─────────                     │  ──────────────────                        │
│  Single refusal direction      │  Monolithic summarization                  │
│  → Decompose into 2 directions │  → Decompose into mask + fallback         │
│  → Targeted per-direction      │  → Targeted per-observation               │
│     intervention               │     management                            │
│  → 97.88% success              │  → 59% cost reduction                     │
│                                │                                            │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  IMPLICATIONS FOR AGENT CONTEXT MANAGEMENT:                                 │
│                                                                             │
│  1. Safety-Aware Context Compression                                        │
│     • Context management strategies must preserve safety-critical           │
│       information that enables harm detection                               │
│     • Aggressive masking/summarization could inadvertently strip            │
│       safety signals from agent trajectories                                │
│     • The bi-direction model suggests safety information exists             │
│       in specific activation patterns, not distributed broadly             │
│                                                                             │
│  2. Adversarial Robustness of Agent Memory                                  │
│     • If safety alignment can be neutralized at a single layer,            │
│       compressed context representations may be equally vulnerable         │
│     • Summarized trajectories may lose the fine-grained activation         │
│       patterns that enable harm detection                                   │
│     • Observation masking (preserving recent full context) may              │
│       better preserve safety-critical signals than lossy compression        │
│                                                                             │
│  3. The "Complexity Trap" in Safety Research                                │
│     • Prior work assumed a single complex safety direction                  │
│     • DBDI shows two simple directions are more effective                   │
│     • Mirrors: prior work assumed complex summarization needed              │
│     • Complexity Trap shows simple masking is equally effective             │
│                                                                             │
│  4. Causal Structure in Agent Trajectories                                  │
│     • DBDI reveals causal ordering matters in safety mechanisms            │
│     • Similarly, agent trajectory management should respect                 │
│       causal dependencies between observations                             │
│     • Masking old observations preserves causal ordering;                   │
│       summarization may flatten it                                          │
│                                                                             │
│  5. Layer-Specific vs. Trajectory-Wide Intervention                         │
│     • DBDI operates at a single critical layer (Layer 16/32)               │
│     • Context management operates across the full trajectory               │
│     • Both benefit from identifying the critical intervention point        │
│     • Hybrid approach: mask early turns, summarize only at                 │
│       critical trajectory inflection points                                 │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Specific Connections

| DBDI Concept | Complexity Trap Parallel | Implication |
|-------------|--------------------------|-------------|
| Bi-direction decomposition | Mask vs. summarize decomposition | Break monolithic approaches into targeted components |
| Causal ordering (detection → execution) | Trajectory ordering (observe → reason → act) | Preserve causal structure in compression |
| Critical layer selection (Layer 16) | Masking threshold selection (M=10) | Single intervention point can be optimal |
| Adaptive projection nullification | Observation masking (adaptive per-turn) | State-dependent intervention outperforms static |
| Cross-dataset transfer | Cross-model generalization | Fundamental mechanisms transfer across contexts |
| Classifier-guided sparsification | Token-count-based masking | Use signal strength to guide what to preserve |

---

## Defensive Implications

### What DBDI Reveals About Safety Fragility

```
┌─────────────────────────────────────────────────────────────────────────────┐
│             DEFENSIVE TAKEAWAYS                                             │
│                                                                             │
│  1. Single-Layer Vulnerability                                              │
│     • Safety alignment concentrated at one critical layer                  │
│     • Suggests need for distributed safety mechanisms across layers        │
│     • Parallel: Single-strategy context management is fragile;             │
│       hybrid approaches distribute robustness                              │
│                                                                             │
│  2. Causal Redundancy Needed                                                │
│     • Detection → Execution is a single causal chain                       │
│     • Adding redundant detection pathways would resist DBDI                │
│     • Parallel: Multiple context preservation strategies                   │
│       (mask + summarize + retrieve) provide redundancy                     │
│                                                                             │
│  3. Activation Monitoring                                                   │
│     • If safety directions are known, they can be monitored                │
│     • Runtime detection of nullification attempts possible                 │
│     • Parallel: Runtime monitoring of context compression quality          │
│       can detect information loss before it causes failures                │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## References

1. Zhang, P. & Sun, P. "Differentiated Directional Intervention: A Framework for Evading LLM Safety Alignment." AAAI-26 AIA, 2025. ([arXiv:2511.06852](https://arxiv.org/abs/2511.06852))
2. Lindenbauer et al., "The Complexity Trap: Simple Observation Masking Is as Efficient as LLM Summarization for Agent Context Management," NeurIPS 2025 DL4C Workshop. ([arXiv:2508.21433](https://arxiv.org/abs/2508.21433))
3. Zou et al., "Universal and Transferable Adversarial Attacks on Aligned Language Models," 2023 (AdvBench).
4. Mazeika et al., "HarmBench: A Standardized Evaluation Framework for Automated Red Teaming and Robust Refusal," 2024.
5. Souly et al., "A StrongREJECT for Empty Jailbreaks," 2024.
6. Pan et al., "Hidden in Plain Text: Emergence & Mitigation of Steganographic Collusion in LLMs," 2025.

---

## Next Steps

- **[Research Summary](../architecture/01-research-summary.md)** — Core findings of the Complexity Trap
- **[Advanced Strategies](../strategies/04-advanced-strategies.md)** — Context management approaches from 2025
- **[Future Directions](../challenges/02-future-work.md)** — Open problems in agent efficiency and safety
- **[Related Research](../related-work/03-related-papers.md)** — Concurrent work on agent context management
