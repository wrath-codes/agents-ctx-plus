# TTT-E2E: End-to-End Test-Time Training for Long Context

## Overview

TTT-E2E reformulates long-context language modeling as a **continual learning** problem rather than an architecture design problem. Instead of relying on architectural innovations like extended attention or recurrent state-space models, TTT-E2E uses a standard Transformer with sliding-window attention (SWA) and continues learning at test time via next-token prediction on the given context — compressing information directly into the model's weights.

**Paper**: "End-to-End Test-Time Training for Long Context" (Tandon et al., arXiv:2512.23675)  
**Authors**: Arnuv Tandon et al.  
**Code**: [github.com/test-time-training/e2e](https://github.com/test-time-training/e2e)  
**Published**: December 2025

## Core Concept

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                     TTT-E2E: CONTEXT AS CONTINUAL LEARNING                  │
│                                                                             │
│  Traditional Approach (Architecture Design):                                │
│  ──────────────────────────────────────────                                 │
│                                                                             │
│  Challenge: How do we attend to 128K tokens?                                │
│  Answer:    Build a better architecture (full attention, Mamba, DeltaNet)   │
│                                                                             │
│  Problem:   Full attention → O(n²) cost, quadratic latency growth          │
│             RNN alternatives → Don't scale with context length             │
│                                                                             │
│  ═══════════════════════════════════════════════════════════════════════    │
│                                                                             │
│  TTT-E2E Approach (Continual Learning):                                     │
│  ──────────────────────────────────────                                     │
│                                                                             │
│  Challenge: How do we use 128K tokens of context?                           │
│  Answer:    Learn from the context at test time via gradient descent        │
│                                                                             │
│  Key Insight: The model reads context tokens and takes gradient steps       │
│               on next-token prediction loss, compressing context into       │
│               its MLP weights — then uses sliding-window attention for      │
│               local context during generation                               │
│                                                                             │
│  Result:    Constant inference latency (like RNNs)                          │
│             Scales with context length (like full attention)                │
│             2.7× faster than full attention at 128K                        │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Key Contributions

1. **Paradigm Shift** — Treats long-context as continual learning rather than architecture design
2. **E2E at Both Phases** — End-to-end at test time (next-token prediction) and training time (meta-learning)
3. **Standard Infrastructure** — Uses regular Transformer MLP layers as hidden state, enabling standard GPU sharding with no custom kernels
4. **Scaling Properties** — For 3B models trained with 164B tokens, scales with context length identically to full attention
5. **Constant Latency** — Like RNNs, inference latency is independent of context length

## Architecture

### Two-Phase Design

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                       TTT-E2E ARCHITECTURE                                  │
│                                                                             │
│  PHASE 1: TRAINING TIME (Meta-Learning)                                     │
│  ──────────────────────────────────────                                     │
│                                                                             │
│  Outer Loop: Standard pre-training on DCLM (8K context)                     │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ For each training batch:                                             │   │
│  │   1. Forward pass through Transformer (SWA + MLP blocks)            │   │
│  │   2. Inner loop: TTT gradient steps on MLP weights                  │   │
│  │   3. Compute outer loss (next-token prediction)                      │   │
│  │   4. Backpropagate through both loops (meta-learning)                │   │
│  │                                                                     │   │
│  │ Result: Model learns a good initialization for test-time learning   │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  PHASE 2: TEST TIME (Continual Learning)                                    │
│  ─────────────────────────────────────                                      │
│                                                                             │
│  Given context of length T (e.g., 128K tokens):                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                                                                     │   │
│  │  Context: [x₁, x₂, ..., x_T]                                       │   │
│  │                                                                     │   │
│  │  Mini-batch 1: [x₁ ... x_b]                                        │   │
│  │    → Forward pass (SWA handles local context)                       │   │
│  │    → Compute next-token prediction loss                             │   │
│  │    → Gradient step on MLP weights (inner loop)                      │   │
│  │    → Context compressed into updated weights                        │   │
│  │                                                                     │   │
│  │  Mini-batch 2: [x_{b+1} ... x_{2b}]                                │   │
│  │    → Same process, building on updated weights                      │   │
│  │    → Previous context "remembered" in MLP parameters                │   │
│  │                                                                     │   │
│  │  ... (repeat for each mini-batch)                                   │   │
│  │                                                                     │   │
│  │  Generation: Use updated MLPs + SWA for next-token prediction       │   │
│  │                                                                     │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Sliding-Window Attention + Test-Time Training

```
┌─────────────────────────────────────────────────────────────────────────────┐
│           SWA + TTT: COMPLEMENTARY MECHANISMS                               │
│                                                                             │
│  Token Stream: x₁  x₂  x₃  x₄  x₅  x₆  x₇  x₈  x₉  x₁₀ ...           │
│                                                                             │
│  SWA (window k=8K):                                                         │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ Handles LOCAL context within window                                 │   │
│  │                                                                     │   │
│  │ At token x₁₀:                                                      │   │
│  │   Attends to: [x₃, x₄, x₅, x₆, x₇, x₈, x₉, x₁₀]  (window=8)   │   │
│  │   Cannot see: [x₁, x₂]  (outside window)                          │   │
│  │                                                                     │   │
│  │ Strength: Precise local attention (exact retrieval within window)   │   │
│  │ Weakness: No access to tokens beyond window                        │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  TTT (mini-batch b=1K):                                                     │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ Handles LONG-RANGE context via weight updates                       │   │
│  │                                                                     │   │
│  │ At token x₁₀:                                                      │   │
│  │   MLP weights encode: Information from [x₁, x₂, ..., x₉]          │   │
│  │   via gradient steps on mini-batches                                │   │
│  │                                                                     │   │
│  │ Strength: Compresses arbitrarily long context into fixed-size state │   │
│  │ Weakness: Lossy compression (not exact retrieval)                   │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  Combined:                                                                  │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ SWA provides: Exact local context (recent tokens)                   │   │
│  │ TTT provides: Compressed global context (all past tokens)           │   │
│  │                                                                     │   │
│  │ Constraint: k ≥ b (window must cover at least one mini-batch)       │   │
│  │ Default:    k = 8K, b = 1K                                         │   │
│  │                                                                     │   │
│  │ Result: Best of both — local precision + global awareness           │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Hidden State as MLP Weights

```
┌─────────────────────────────────────────────────────────────────────────────┐
│              HIDDEN STATE DESIGN                                            │
│                                                                             │
│  Traditional RNNs:                                                          │
│  ─────────────────                                                          │
│  • Custom hidden state format                                               │
│  • Requires custom kernels (e.g., Mamba's selective scan)                   │
│  • Must fit on individual GPU chips                                         │
│  • Limited state size → limited compression capacity                       │
│                                                                             │
│  TTT-E2E:                                                                   │
│  ────────                                                                   │
│  • Hidden state = last L/4 MLP layers of the Transformer                   │
│  • Standard weight format → standard GPU sharding                          │
│  • No custom kernels needed                                                 │
│  • State size scales with model size                                        │
│                                                                             │
│  ┌─────────────────────────────────────┐                                    │
│  │  Transformer Blocks (760M model)    │                                    │
│  │                                     │                                    │
│  │  Block 1:  [SWA] [MLP] ← Frozen    │                                    │
│  │  Block 2:  [SWA] [MLP] ← Frozen    │                                    │
│  │  ...                                │                                    │
│  │  Block 18: [SWA] [MLP] ← Frozen    │                                    │
│  │  ─────────────────────────────────  │                                    │
│  │  Block 19: [SWA] [MLP] ← Updated   │  Hidden state:                     │
│  │  Block 20: [SWA] [MLP] ← Updated   │  88M parameters                    │
│  │  Block 21: [SWA] [MLP] ← Updated   │  (last 1/4 of blocks)             │
│  │  Block 22: [SWA] [MLP] ← Updated   │                                    │
│  │  Block 23: [SWA] [MLP] ← Updated   │  5× larger than                    │
│  │  Block 24: [SWA] [MLP] ← Updated   │  TTT-KVB (18M)                     │
│  │                                     │                                    │
│  └─────────────────────────────────────┘                                    │
│                                                                             │
│  Frozen during TTT: Embedding layers, normalization layers,                 │
│                     attention layers, first 3/4 of MLP layers              │
│  Updated during TTT: Last 1/4 of MLP layers only                          │
│                                                                             │
│  Key Insight: Updating fewer blocks with larger state is more              │
│               cost-effective than updating many blocks with smaller state   │
│               because gradients must back-propagate through expensive       │
│               upstream layers regardless                                    │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Methodology

### Mini-Batch TTT Process

The model does not perform TTT on every token individually. Instead, it accumulates tokens into mini-batches of size `b` and takes a gradient step per batch.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    MINI-BATCH TTT PROCESS                                    │
│                                                                             │
│  Context: [x₁, x₂, x₃, ... , x₁₂₈₀₀₀]  (128K tokens)                    │
│                                                                             │
│  Step 1: Fill mini-batch 1                                                  │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ [x₁, x₂, ..., x₁₀₀₀]  (b=1K tokens)                              │   │
│  │                                                                     │   │
│  │ SWA processes local context (window k=8K)                           │   │
│  │ No TTT yet — SWA handles memory within first batch                 │   │
│  │                                                                     │   │
│  │ After batch completes:                                              │   │
│  │   Loss = Σ ℓ(x_t, p̂_t) for t in [1, 1000]                        │   │
│  │   θ_MLP ← θ_MLP - η · ∇_θ Loss    (gradient step)                │   │
│  │   Context [x₁..x₁₀₀₀] now compressed into θ_MLP                   │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  Step 2: Fill mini-batch 2                                                  │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ [x₁₀₀₁, x₁₀₀₂, ..., x₂₀₀₀]  (next b=1K tokens)                  │   │
│  │                                                                     │   │
│  │ SWA: local attention within window                                  │   │
│  │ Updated θ_MLP: carries information from mini-batch 1               │   │
│  │                                                                     │   │
│  │ After batch completes:                                              │   │
│  │   Loss = Σ ℓ(x_t, p̂_t) for t in [1001, 2000]                     │   │
│  │   θ_MLP ← θ_MLP - η · ∇_θ Loss    (another gradient step)        │   │
│  │   Context [x₁..x₂₀₀₀] now compressed into θ_MLP                   │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  ... repeat for 128 mini-batches (128K / 1K = 128 steps) ...               │
│                                                                             │
│  Step 128: Final mini-batch                                                 │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ [x₁₂₇₀₀₁, ..., x₁₂₈₀₀₀]                                         │   │
│  │                                                                     │   │
│  │ θ_MLP now encodes information from entire 128K context              │   │
│  │ SWA provides exact attention over last 8K tokens                   │   │
│  │                                                                     │   │
│  │ Model ready for generation with full context awareness              │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  Total TTT gradient steps: T/b = 128K/1K = 128 steps                      │
│  Each step: One forward + backward pass through last L/4 blocks            │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Meta-Learning at Training Time

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                META-LEARNING (LEARNING TO LEARN AT TEST TIME)                │
│                                                                             │
│  Naive TTT:                                                                 │
│  ──────────                                                                 │
│  • Standard pre-training, then do TTT at test time                         │
│  • Model not prepared for test-time weight updates                          │
│  • Result: Only slightly better than no TTT at all                         │
│                                                                             │
│  TTT-E2E Meta-Learning:                                                     │
│  ──────────────────────                                                     │
│  • Training explicitly optimizes for test-time learning ability            │
│  • Outer loop loss: next-token prediction on DCLM data                     │
│  • Inner loop: TTT gradient steps on MLP weights                           │
│  • Gradients flow through BOTH loops (end-to-end)                          │
│                                                                             │
│  Training Objective:                                                        │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                                                                     │   │
│  │  min_θ  L_outer(θ*)                                                 │   │
│  │                                                                     │   │
│  │  where θ* = θ - η_inner · ∇_θ L_inner(θ)                          │   │
│  │                                                                     │   │
│  │  L_inner = next-token prediction loss on context (TTT)              │   │
│  │  L_outer = next-token prediction loss after TTT                     │   │
│  │  η_inner = inner learning rate (for test-time updates)              │   │
│  │                                                                     │   │
│  │  Key: ∂L_outer/∂θ requires differentiating through θ*,             │   │
│  │       which requires "gradients of gradients"                       │   │
│  │                                                                     │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  Result: Model initialized to learn effectively from context at test time  │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Implementation

### Core TTT-E2E Algorithm

```python
class TTTE2E:
    """
    TTT-E2E: End-to-End Test-Time Training for Long Context.
    
    Compresses long context into MLP weights via gradient descent
    at test time, using sliding-window attention for local context.
    
    Architecture: Standard Transformer with SWA + TTT on last L/4 MLPs.
    """
    
    def __init__(
        self,
        model,
        window_size_k: int = 8192,
        mini_batch_size_b: int = 1024,
        ttt_learning_rate: float = 1e-4,
        num_ttt_layers_fraction: float = 0.25,
    ):
        self.model = model
        self.k = window_size_k
        self.b = mini_batch_size_b
        self.ttt_lr = ttt_learning_rate
        
        total_blocks = len(model.transformer_blocks)
        self.ttt_start_layer = int(total_blocks * (1 - num_ttt_layers_fraction))
        
        self.ttt_params = self._get_ttt_parameters()
        self.frozen_params = self._get_frozen_parameters()
    
    def _get_ttt_parameters(self) -> list:
        """Get MLP parameters from last L/4 blocks (updated during TTT)."""
        ttt_params = []
        for i, block in enumerate(self.model.transformer_blocks):
            if i >= self.ttt_start_layer:
                ttt_params.extend(block.mlp.parameters())
        return ttt_params
    
    def _get_frozen_parameters(self) -> list:
        """Get all parameters NOT updated during TTT."""
        frozen = []
        for i, block in enumerate(self.model.transformer_blocks):
            frozen.extend(block.attention.parameters())
            frozen.extend(block.norm.parameters())
            if i < self.ttt_start_layer:
                frozen.extend(block.mlp.parameters())
        frozen.extend(self.model.embedding.parameters())
        return frozen
    
    def prefill(self, context_tokens: list) -> dict:
        """
        Process context via TTT: compress into MLP weights.
        
        Args:
            context_tokens: List of token IDs (can be 128K+)
        
        Returns:
            State dict with updated MLP weights and SWA cache
        """
        original_weights = self._save_ttt_weights()
        
        num_batches = len(context_tokens) // self.b
        remainder = len(context_tokens) % self.b
        
        ttt_steps = 0
        
        for batch_idx in range(num_batches):
            start = batch_idx * self.b
            end = start + self.b
            mini_batch = context_tokens[start:end]
            
            loss = self._compute_ntp_loss(mini_batch)
            
            grads = self._compute_gradients(loss, self.ttt_params)
            
            self._update_weights(self.ttt_params, grads, self.ttt_lr)
            
            ttt_steps += 1
        
        if remainder > 0:
            remaining = context_tokens[num_batches * self.b:]
            self.swa_cache = self._build_swa_cache(remaining)
        
        return {
            'ttt_steps': ttt_steps,
            'context_length': len(context_tokens),
            'hidden_state_size': sum(p.numel() for p in self.ttt_params),
            'original_weights': original_weights,
        }
    
    def _compute_ntp_loss(self, tokens: list) -> float:
        """
        Compute next-token prediction loss for a mini-batch.
        
        The model uses SWA for local attention within the batch,
        and the (potentially updated) MLP weights for global context.
        """
        inputs = tokens[:-1]
        targets = tokens[1:]
        
        logits = self.model.forward(
            inputs,
            attention_mask='sliding_window',
            window_size=self.k
        )
        
        loss = cross_entropy(logits, targets)
        return loss
    
    def _compute_gradients(self, loss, params):
        """Compute gradients of loss w.r.t. TTT parameters only."""
        return autograd.grad(loss, params)
    
    def _update_weights(self, params, grads, lr):
        """Apply gradient descent step to TTT parameters."""
        for param, grad in zip(params, grads):
            param.data -= lr * grad
    
    def decode(self, prompt_tokens: list, max_new_tokens: int) -> list:
        """
        Generate tokens using updated MLP weights + SWA.
        
        Decode latency is constant regardless of original context length,
        since context is compressed into weights.
        """
        generated = []
        current_tokens = prompt_tokens[-self.k:]
        
        batch_buffer = []
        
        for _ in range(max_new_tokens):
            logits = self.model.forward(
                current_tokens,
                attention_mask='sliding_window',
                window_size=self.k
            )
            
            next_token = self._sample(logits[-1])
            generated.append(next_token)
            
            current_tokens = current_tokens[1:] + [next_token]
            
            batch_buffer.append(next_token)
            if len(batch_buffer) == self.b:
                loss = self._compute_ntp_loss(batch_buffer)
                grads = self._compute_gradients(loss, self.ttt_params)
                self._update_weights(self.ttt_params, grads, self.ttt_lr)
                batch_buffer = []
        
        return generated
    
    def reset(self, original_weights: dict):
        """Reset MLP weights to pre-TTT state."""
        self._restore_ttt_weights(original_weights)


class TTTE2EMetaTrainer:
    """
    Meta-learning trainer for TTT-E2E.
    
    Optimizes model initialization for effective test-time learning
    by differentiating through the inner TTT loop during training.
    """
    
    def __init__(
        self,
        model,
        outer_lr: float = 4e-4,
        inner_lr: float = 1e-4,
        inner_steps: int = 1,
        mini_batch_size: int = 1024,
    ):
        self.model = model
        self.outer_lr = outer_lr
        self.inner_lr = inner_lr
        self.inner_steps = inner_steps
        self.b = mini_batch_size
        
        self.outer_optimizer = AdamW(
            model.parameters(), lr=outer_lr
        )
    
    def train_step(self, sequence: list) -> dict:
        """
        One meta-learning training step.
        
        1. Split sequence into context and target
        2. Inner loop: TTT on context (update MLP weights)
        3. Outer loop: Compute loss on target, backprop through inner loop
        """
        context = sequence[:len(sequence) // 2]
        target = sequence[len(sequence) // 2:]
        
        saved_weights = self._save_ttt_weights()
        
        for step in range(self.inner_steps):
            for i in range(0, len(context), self.b):
                batch = context[i:i + self.b]
                inner_loss = self._compute_ntp_loss(batch)
                
                grads = torch.autograd.grad(
                    inner_loss,
                    self.model.ttt_params,
                    create_graph=True
                )
                
                for param, grad in zip(self.model.ttt_params, grads):
                    param.data = param.data - self.inner_lr * grad
        
        outer_loss = self._compute_ntp_loss(target)
        
        self.outer_optimizer.zero_grad()
        outer_loss.backward()
        self.outer_optimizer.step()
        
        self._restore_ttt_weights(saved_weights)
        
        return {
            'outer_loss': outer_loss.item(),
            'inner_loss': inner_loss.item(),
        }
```

### Latency Characteristics

```python
def estimate_ttt_e2e_latency(
    context_length: int,
    mini_batch_size: int = 1024,
    window_size: int = 8192,
    prefill_per_1k: float = 0.0086,
    swa_decode_per_token: float = 0.00001,
) -> dict:
    """
    Estimate TTT-E2E inference latency.
    
    Key property: Constant decode latency regardless of context length.
    Prefill latency grows linearly (not quadratically) with context.
    
    Args:
        context_length: Number of context tokens
        mini_batch_size: TTT mini-batch size (b)
        window_size: SWA window size (k)
        prefill_per_1k: Seconds per 1K tokens for prefill (H100)
        swa_decode_per_token: Seconds per decoded token (SWA only)
    
    Reference (3B model on H100):
        TTT-E2E prefill: 0.0086 sec/1K tokens
        Full attention:  scales quadratically
        At 128K context: TTT-E2E is 2.7× faster
    """
    num_ttt_steps = context_length // mini_batch_size
    prefill_latency = (context_length / 1000) * prefill_per_1k
    
    decode_latency_per_token = swa_decode_per_token
    
    ttt_update_per_batch = prefill_per_1k * (mini_batch_size / 1000)
    
    full_attention_prefill = (context_length / 1000) ** 2 * 0.001
    
    return {
        'prefill_latency_sec': prefill_latency,
        'decode_latency_per_token_sec': decode_latency_per_token,
        'ttt_steps': num_ttt_steps,
        'full_attention_prefill_sec': full_attention_prefill,
        'speedup_vs_full_attention': full_attention_prefill / prefill_latency,
        'context_length': context_length,
        'decode_latency_constant': True,
    }
```

## Results

### Language Modeling Performance (760M Models)

| Method | Loss (DCLM) | Diff. vs SWA |
|--------|------------:|-------------:|
| SWA (k=8K) baseline | 2.827 | — |
| TTT-KVB (Zhang et al.) | 2.818 | -0.009 |
| TTT-KVB simplified | 2.819 | -0.008 |
| TTT-E2E all layers MH | 2.806 | -0.021 |
| **TTT-E2E (ours)** | **2.805** | **-0.022** |

### Context Length Scaling (3B Models, 164B Training Tokens)

| Method | Scales with Context? | Behavior at 128K |
|--------|---------------------|-----------------|
| **Full Attention** | Yes | Baseline (quadratic cost) |
| **TTT-E2E** | **Yes** | **Matches full attention** |
| Mamba 2 | No | Degrades |
| Gated DeltaNet | No | Degrades |
| SWA (k=8K) | No | Degrades severely |
| TTT-KVB | Partial | Some degradation |

TTT-E2E is the only sub-quadratic method that maintains the same scaling behavior as full attention across all tested context lengths (8K to 128K).

### Inference Latency (3B Model, H100 GPU)

| Method | Latency Scaling | At 128K Context | Relative Speed |
|--------|----------------|-----------------|---------------|
| Full Attention | Quadratic (O(n²)) | Baseline | 1.0× |
| SWA | Constant | Fast | ~2.7× |
| Mamba 2 | Constant | Fast | ~2.7× |
| Gated DeltaNet | Constant | Fast | ~2.7× |
| **TTT-E2E** | **Constant** | **Fast** | **2.7×** |

### Hidden State Comparison (760M Model)

| Method | Hidden State Size | Prefill Latency (sec/1K) | Custom Kernels |
|--------|------------------:|-------------------------:|:--------------:|
| TTT-KVB (multi-head + LoRA) | 18M | 0.017 | Required |
| **TTT-E2E (regular MLP)** | **88M** | **0.0086** | **None** |

TTT-E2E achieves a 5× larger hidden state at half the prefill latency by updating fewer blocks with larger state rather than many blocks with smaller state.

### Needle-in-a-Haystack (RULER S-NIAH, 3B Models)

| Method | 8K | 16K | 32K | 64K | 128K |
|--------|---:|----:|----:|----:|-----:|
| Full Attention | 1.00 | 1.00 | 1.00 | 1.00 | 0.99 |
| SWA | 1.00 | 0.50 | 0.26 | 0.13 | 0.07 |
| Mamba 2 | 0.99 | 0.49 | 0.26 | 0.13 | 0.07 |
| Gated DeltaNet | 1.00 | 0.50 | 0.26 | 0.13 | 0.07 |
| TTT-KVB | 0.98 | 0.43 | 0.22 | 0.10 | 0.01 |
| TTT-E2E | 1.00 | 0.46 | 0.24 | 0.13 | 0.06 |

**Important Note**: On S-NIAH (passkey retrieval), TTT-E2E performs similarly to other sub-quadratic methods — all significantly below full attention. This reflects the known limitation that lossy compression cannot match exact retrieval for needle-in-a-haystack tasks. However, TTT-E2E excels on the **language modeling** metric, which better captures natural long-range dependency utilization.

### Ablation Results (760M Models)

| Hyperparameter | Value | Effect |
|---------------|-------|--------|
| Window size k | 2K → 8K | Larger k improves all methods similarly |
| Mini-batch b | 1K (optimal) | b > 1K significantly hurts performance |
| | | b < 1K hurts hardware utilization and stability |
| Layers updated | Last 1/4 (optimal) | More layers → more compute, diminishing returns |
| | | Fewer layers → insufficient state for compression |
| Without TTT (b=8K) | Loss: 2.825 | Almost identical to full attention (2.827) |
| | | Architecture modifications alone have minimal effect |

## Connection to the Complexity Trap

### How TTT-E2E Addresses Context Bloat

```
┌─────────────────────────────────────────────────────────────────────────────┐
│          TTT-E2E AND THE COMPLEXITY TRAP                                    │
│                                                                             │
│  The Complexity Trap (Lindenbauer et al., 2025):                            │
│  ──────────────────────────────────────────────                             │
│  "Simple observation masking matches LLM summarization at lower cost"      │
│                                                                             │
│  Core Problem: Agent trajectories grow without bound, and sophisticated     │
│  compression (LLM summarization) doesn't outperform simple omission        │
│  (observation masking) — but both suffer from growing context costs.        │
│                                                                             │
│  ═══════════════════════════════════════════════════════════════════════    │
│                                                                             │
│  TTT-E2E Addresses a DIFFERENT Level of the Stack:                          │
│  ─────────────────────────────────────────────────                          │
│                                                                             │
│  Complexity Trap operates at: TRAJECTORY MANAGEMENT LAYER                   │
│    "What context to keep/mask/summarize?"                                   │
│    → Observation masking, LLM summarization, hybrid                        │
│                                                                             │
│  TTT-E2E operates at: MODEL INFERENCE LAYER                                │
│    "How does the model process whatever context it receives?"              │
│    → Compress context into weights, constant-time inference                │
│                                                                             │
│  ═══════════════════════════════════════════════════════════════════════    │
│                                                                             │
│  COMPLEMENTARY, NOT COMPETING:                                              │
│                                                                             │
│  Stack with TTT-E2E:                                                        │
│  ┌───────────────────────────────────────────────────────────────────┐     │
│  │ Layer 3: Agent Policy (reasoning, planning)                       │     │
│  ├───────────────────────────────────────────────────────────────────┤     │
│  │ Layer 2: Context Management (masking/hybrid from Complexity Trap) │     │
│  ├───────────────────────────────────────────────────────────────────┤     │
│  │ Layer 1: Model Inference (TTT-E2E for efficient long context)     │     │
│  └───────────────────────────────────────────────────────────────────┘     │
│                                                                             │
│  Combined benefit:                                                          │
│  • Layer 2 reduces what the model sees (fewer tokens, lower cost)          │
│  • Layer 1 processes what remains more efficiently (constant latency)      │
│  • Together: multiplicative efficiency gains                               │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Impact on Key Complexity Trap Findings

| Finding | TTT-E2E Implication |
|---------|---------------------|
| **Context management is essential** | TTT-E2E doesn't eliminate this need — even with efficient inference, trajectory management remains necessary to reduce API costs and improve signal-to-noise |
| **Simple beats sophisticated** | TTT-E2E is itself a "simple" approach — standard architecture, standard infrastructure, no custom kernels. Validates the Complexity Trap principle at the model layer |
| **Trajectory elongation** | TTT-E2E's constant latency makes elongation less costly per turn, but doesn't prevent it. Trajectory management strategies (masking/hybrid) still needed |
| **Hybrid wins** | TTT-E2E + hybrid context management could represent the optimal combined strategy: efficient inference + intelligent trajectory management |

### Relevance to Agent Deployment

```
┌─────────────────────────────────────────────────────────────────────────────┐
│              AGENT COST EQUATION WITH TTT-E2E                               │
│                                                                             │
│  Current (Full Attention + No Management):                                  │
│  ──────────────────────────────────────────                                 │
│  Cost = Σ (context_tokens_t × price_per_token)                             │
│  Latency = Σ O(context_tokens_t²)                                          │
│  Both grow quadratically → economically infeasible                         │
│                                                                             │
│  With Hybrid Context Management (Complexity Trap):                          │
│  ────────────────────────────────────────────────                           │
│  Cost = Σ (managed_tokens_t × price_per_token)                             │
│  Savings: ~50-59% cost reduction                                           │
│  But: latency still quadratic in managed context                           │
│                                                                             │
│  With TTT-E2E (Model Layer):                                                │
│  ─────────────────────────────                                              │
│  Cost = prefill_cost + Σ (constant_decode_cost)                            │
│  Latency = constant per token regardless of context                        │
│  Savings: 2.7× latency reduction at 128K                                  │
│                                                                             │
│  With Both (Optimal Stack):                                                 │
│  ──────────────────────────                                                 │
│  Cost = managed_prefill + Σ (constant_decode)                              │
│  Latency = constant                                                         │
│  Savings: Multiplicative — 50%+ cost × 2.7× latency                       │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Limitations

### Current Constraints

| Limitation | Details | Severity |
|-----------|---------|----------|
| **Training latency** | Cannot use cuDNN FlashAttention (no "gradients of gradients" support); 3.2× slower than full attention training | High |
| **Needle-in-a-haystack** | Lossy compression cannot match exact retrieval for precise lookup tasks | Medium |
| **Mini-batch granularity** | Context must accumulate b=1K tokens before first TTT step; b < 1K causes instability | Low |
| **Pre-training requirement** | Requires meta-learning during pre-training; cannot be applied to existing models without retraining | High |
| **Short context regime** | Below 8K tokens, TTT overhead provides no benefit (SWA alone suffices) | Low |

### Comparison with Complexity Trap Strategies

| Dimension | Observation Masking | TTT-E2E |
|-----------|---------------------|---------|
| **Deployment ease** | Drop-in, any model | Requires TTT-E2E trained model |
| **Infrastructure** | No changes needed | Standard (no custom kernels) |
| **Training cost** | None | Significant (meta-learning) |
| **Inference cost** | Reduces token count | Reduces latency per token |
| **Information loss** | Old observations hidden | Lossy compression into weights |
| **Complementary?** | — | Yes, operates at different layer |

## Experimental Setup

### Model Configurations

| Model Size | Training Tokens | Pre-training Data | Fine-tuning Data |
|-----------:|----------------:|:-----------------:|:----------------:|
| 125M | Varies | DCLM | Books |
| 350M | Varies | DCLM | Books |
| 760M | 48B (basic) | DCLM | Books |
| 1B | Varies | DCLM | Books |
| 3B | 164B (3× basic) | DCLM | Books |

### Key Hyperparameters

| Parameter | Value | Rationale |
|-----------|-------|-----------|
| SWA window size (k) | 8K | Best tradeoff; smaller doesn't significantly improve runtime |
| TTT mini-batch size (b) | 1K | b > 1K hurts performance; b < 1K hurts stability |
| Layers updated | Last 1/4 | Optimal state size vs. compute tradeoff |
| Pre-training context | 8K | Standard for base model training |
| Fine-tuning context | Up to 128K | Extension via Books dataset |
| Fine-tuning tokens | 5% of pre-training | Standard recipe |
| Learning rate | 4e-4 | Best across all model sizes and context lengths |

## Code and Data Availability

- **Paper**: [arXiv:2512.23675](https://arxiv.org/abs/2512.23675)
- **Code**: [github.com/test-time-training/e2e](https://github.com/test-time-training/e2e)
- **Framework**: JAX (with PyTorch baselines for comparison)

## Next Steps

- **[Observation Masking](01-observation-masking.md)** — Complementary trajectory-level strategy
- **[Hybrid Approach](03-hybrid-approach.md)** — Optimal trajectory management for use with TTT-E2E
- **[Advanced Strategies](04-advanced-strategies.md)** — Other 2025 context management innovations
- **[Trajectory Elongation](../experiments/03-trajectory-elongation.md)** — Why constant latency matters
- **[Future Work](../challenges/02-future-work.md)** — Open problems in context efficiency
