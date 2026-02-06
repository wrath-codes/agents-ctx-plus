# PLENA: Hardware-Software Co-Design for Long-Context Agentic LLM Inference

## Overview

**Paper**: "Combating the Memory Walls: Optimization Pathways for Long-Context Agentic LLM Inference"  
**Authors**: Haoran Wu et al.  
**Link**: [arXiv:2509.09505](https://arxiv.org/abs/2509.09505)  
**Subject**: Hardware Architecture (cs.AR)  
**Status**: Open-source release planned

PLENA (**P**rogrammable **L**ong-context **E**fficient **N**eural **A**ccelerator) is a hardware-software co-designed system specifically engineered to address the memory bottlenecks encountered during long-context agentic LLM inference. Unlike chatbot-focused inference where contexts are short, agentic workloads — such as processing entire webpage DOMs or complex tool call trajectories — generate massive context histories that overwhelm existing accelerator architectures.

## The Two Memory Walls

Agentic LLM inference encounters two distinct memory bottlenecks that prevent on-chip compute units from achieving high utilization:

```
┌─────────────────────────────────────────────────────────────────┐
│                    THE TWO MEMORY WALLS                          │
│                                                                     │
│  Wall 1: BANDWIDTH MEMORY WALL                                     │
│  ──────────────────────────────                                     │
│  Problem: Significant off-chip memory traffic during inference      │
│                                                                     │
│  ┌──────────┐    ← Bandwidth limited →    ┌──────────────┐       │
│  │  On-Chip  │ ◄─────────────────────────► │   HBM        │       │
│  │  Compute  │    Data transfer too slow   │   (Off-Chip)  │       │
│  │  Units    │    for long contexts        │   Memory      │       │
│  └──────────┘                               └──────────────┘       │
│                                                                     │
│  Impact: Compute units idle waiting for data                        │
│  Cause:  Long contexts → large KV cache → frequent HBM access     │
│                                                                     │
│  Wall 2: CAPACITY MEMORY WALL                                       │
│  ──────────────────────────────                                     │
│  Problem: Memory capacity constraints limit batch size/context     │
│                                                                     │
│  ┌──────────────────────────────────────────────────────────┐     │
│  │  HBM Capacity:                                            │     │
│  │  ┌────────┐ ┌────────┐ ┌────────┐ ┌────────────────┐    │     │
│  │  │ Model  │ │ KV     │ │ Activs │ │  Remaining     │    │     │
│  │  │ Weights│ │ Cache  │ │        │ │  (small!)      │    │     │
│  │  │        │ │ (HUGE) │ │        │ │                │    │     │
│  │  └────────┘ └────────┘ └────────┘ └────────────────┘    │     │
│  │              ↑                                            │     │
│  │  Long-context KV cache consumes most HBM capacity         │     │
│  └──────────────────────────────────────────────────────────┘     │
│                                                                     │
│  Impact: Cannot fit large batches → low throughput                  │
│  Cause:  KV cache scales with context length × layers × heads      │
│                                                                     │
│  Combined Effect:                                                    │
│  Standard systolic arrays (e.g., TPUs): Under-utilized             │
│  Square arrays optimized for "thin" GEMM (small inner dim)         │
│  Agentic workloads produce "fat" GEMM (large inner dim)            │
│  → Mismatch → Low utilization → Wasted compute                     │
└─────────────────────────────────────────────────────────────────────┘
```

### Why Agentic Inference Is Different

| Property | Chatbot Inference | Agentic Inference |
|----------|-------------------|-------------------|
| Context Length | Short (1K-4K tokens) | Long (5K-128K+ tokens) |
| Generation Length | Long (hundreds of tokens) | Variable (8 to 8K+ tokens) |
| KV Cache Size | Small | Massive |
| GEMM Shape | Thin (small inner dim) | Fat (large inner dim) |
| Memory Pressure | Low | High (both walls active) |
| Workload Pattern | Prompt=short, Gen=long | Prompt=long, Gen=variable |
| Example | Chat response | Tool call trajectory, DOM parsing |

### Utilization Impact

Standard square systolic arrays (as used in TPUs) are optimized for thin GEMM operations common in chatbot inference. When confronted with the fat GEMM operations of agentic workloads:

```
Utilization Comparison (LLaMA-3.3-70B, 128K Context):
┌─────────────────────────────────────────────────────────────────┐
│                                                                     │
│  Standard Workload (Prompt=1K, Gen=128):                            │
│  ┌──────────────────────────────────────┐                          │
│  │ Square Systolic Array: ██████████  ~High utilization             │
│  │ PLENA Flattened Array: ██████████  ~High utilization             │
│  └──────────────────────────────────────┘                          │
│  (Both architectures perform well on standard workloads)            │
│                                                                     │
│  Agentic Workload (Prompt=5.6K, Gen=8K):                            │
│  ┌──────────────────────────────────────┐                          │
│  │ Square Systolic Array: █░░░░░░░░░  ~Low utilization              │
│  │ PLENA Flattened Array: ████████░░  ~8.5× higher utilization     │
│  └──────────────────────────────────────┘                          │
│  (PLENA's architecture matches the fat GEMM workload shape)        │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

## Three Optimization Pathways

PLENA addresses the memory walls through three complementary optimization pathways:

```
┌─────────────────────────────────────────────────────────────────┐
│                PLENA OPTIMIZATION PATHWAYS                        │
│                                                                     │
│  Pathway 1: Flattened Systolic Array ──► Bandwidth Wall           │
│  ─────────────────────────────────────                             │
│  • Reshape compute array to match fat GEMM workloads               │
│  • 8 × 512 flattened vs. 64 × 64 square (same multiplier count)   │
│  • Higher utilization for long-context operations                  │
│                                                                     │
│  Pathway 2: Asymmetric Quantization ──► Capacity Wall             │
│  ─────────────────────────────────────                             │
│  • Mixed data types: MXINT, MXFP, MiniFloat                        │
│  • Independent precision per tensor: W (weights), ACT, KV          │
│  • Reduces memory footprint → larger batches → higher throughput   │
│                                                                     │
│  Pathway 3: Native FlashAttention ──► Both Walls                   │
│  ─────────────────────────────────────                             │
│  • Custom ISA instructions for fused attention                      │
│  • Persistent tile-by-tile scheduling on-chip                      │
│  • Eliminates materialization of full attention matrix              │
│                                                                     │
│  Combined Effect:                                                    │
│  • 8.5× higher utilization than existing accelerators               │
│  • 2.24× throughput vs. NVIDIA A100 GPU                            │
│  • 3.85× throughput vs. Google TPU v6e                             │
└─────────────────────────────────────────────────────────────────────┘
```

### Pathway 1: Flattened Systolic Array Architecture

The core architectural innovation is reshaping the systolic array from a square (e.g., 64×64) to a flattened layout (e.g., 8×512) while preserving the same total multiplier count.

```
Standard Square Systolic Array (e.g., TPU):
┌─────────────────────────────────────────────────────────────────┐
│                                                                     │
│   64 × 64 = 4,096 PEs                                              │
│                                                                     │
│   ┌──┬──┬──┬──┬──┬──┬──┬──┐   (64 columns)                        │
│   │PE│PE│PE│PE│PE│PE│PE│..│                                        │
│   ├──┼──┼──┼──┼──┼──┼──┼──┤                                        │
│   │PE│PE│PE│PE│PE│PE│PE│..│                                        │
│   ├──┼──┼──┼──┼──┼──┼──┼──┤   64 rows                              │
│   │PE│PE│PE│PE│PE│PE│PE│..│                                        │
│   ├──┼──┼──┼──┼──┼──┼──┼──┤                                        │
│   │..│..│..│..│..│..│..│..│                                        │
│   └──┴──┴──┴──┴──┴──┴──┴──┘                                        │
│                                                                     │
│   Optimized for: C = A × B where inner dim ≈ 64                   │
│   Problem: Fat GEMM (inner dim >> 64) → low utilization            │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘

PLENA Flattened Systolic Array:
┌─────────────────────────────────────────────────────────────────┐
│                                                                     │
│   8 × 512 = 4,096 PEs (same total multiplier count)                │
│                                                                     │
│   ┌──┬──┬──┬──┬──┬──┬──┬──┬──┬──┬──┬──┬──┬──┬──┬──┬───────┐     │
│   │PE│PE│PE│PE│PE│PE│PE│PE│PE│PE│PE│PE│PE│PE│PE│PE│...×512 │     │
│   ├──┼──┼──┼──┼──┼──┼──┼──┼──┼──┼──┼──┼──┼──┼──┼──┼───────┤     │
│   │PE│PE│PE│PE│PE│PE│PE│PE│PE│PE│PE│PE│PE│PE│PE│PE│...×512 │     │
│   ├──┼──┼──┼──┼──┼──┼──┼──┼──┼──┼──┼──┼──┼──┼──┼──┼───────┤     │
│   │..│..│..│..│..│..│..│..│..│..│..│..│..│..│..│..│...×512 │     │
│   ├──┼──┼──┼──┼──┼──┼──┼──┼──┼──┼──┼──┼──┼──┼──┼──┼───────┤     │
│   │PE│PE│PE│PE│PE│PE│PE│PE│PE│PE│PE│PE│PE│PE│PE│PE│...×512 │     │
│   └──┴──┴──┴──┴──┴──┴──┴──┴──┴──┴──┴──┴──┴──┴──┴──┴───────┘     │
│                          8 rows                                      │
│                                                                     │
│   Optimized for: Fat GEMM where inner dim >> 64                    │
│   Built from: Series of small square sub-arrays (sub-arrs)         │
│   Each PE: Multiply-accumulate, passes data right and down          │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

**Why Flattening Works for Agentic Workloads**:

In agentic inference, the dominant operations involve large inner dimensions (the "fat" dimension of GEMM). A flattened array aligns the long axis with this inner dimension, enabling:

1. **Higher data reuse** — More PEs process the same data stream
2. **Fewer partial sum accumulations** — Reduced inter-tile communication
3. **Better bandwidth utilization** — Streaming matches HBM access patterns

### Pathway 2: Asymmetric Quantization Scheme

PLENA supports independent quantization of weights (QW), activations (QACT), and KV cache (QKV) using different data types and precisions:

```
Asymmetric Quantization Architecture:
┌─────────────────────────────────────────────────────────────────┐
│                                                                     │
│  Supported Data Types:                                              │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐               │
│  │ MXINT       │  │ MXFP        │  │ MiniFloat   │               │
│  │ (Microscale │  │ (Microscale │  │ (Compact    │               │
│  │  Integer)   │  │  Float)     │  │  Float)     │               │
│  └─────────────┘  └─────────────┘  └─────────────┘               │
│                                                                     │
│  Per-Tensor Configuration:                                          │
│  ┌────────────────────────────────────────────────────────────┐   │
│  │ Weights (QW):                                               │   │
│  │   • Most aggressively quantized (4-bit typical)              │   │
│  │   • Static, loaded once → highest compression benefit        │   │
│  │   • L2-Norm-Guided Hessian-Based quantization algorithm      │   │
│  │                                                               │   │
│  │ Activations (QACT):                                           │   │
│  │   • Moderate precision (4-8 bit)                               │   │
│  │   • Dynamic range requires careful handling                   │   │
│  │   • Stored in Vector SRAM scratchpad                          │   │
│  │                                                               │   │
│  │ KV Cache (QKV):                                                │   │
│  │   • Critical for capacity wall                                │   │
│  │   • Lower precision frees HBM for larger batches             │   │
│  │   • 4-bit KV cache → 4× more context in same memory          │   │
│  └────────────────────────────────────────────────────────────┘   │
│                                                                     │
│  Example Configuration (W4A4KV4):                                   │
│  • Weights: 4-bit MXINT                                            │
│  • Activations: 4-bit MXFP                                        │
│  • KV Cache: 4-bit MiniFloat                                       │
│  • Effect: ~4× memory reduction across all tensors                 │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

**L2-Norm-Guided Hessian-Based Weight Quantization**:

```python
class PLENAQuantizer:
    """
    Simplified representation of PLENA's asymmetric quantization.
    
    Supports independent precision selection for weights,
    activations, and KV cache tensors.
    """
    
    SUPPORTED_TYPES = ['MXINT', 'MXFP', 'MiniFloat']
    SUPPORTED_BITS = [4, 8, 16]
    
    def __init__(self, qw_config, qact_config, qkv_config):
        self.qw = qw_config      # e.g., {'type': 'MXINT', 'bits': 4}
        self.qact = qact_config   # e.g., {'type': 'MXFP', 'bits': 4}
        self.qkv = qkv_config     # e.g., {'type': 'MiniFloat', 'bits': 4}
    
    def estimate_memory_reduction(self, model_params, context_length,
                                   num_layers, num_heads, head_dim):
        """Estimate HBM savings from asymmetric quantization."""
        fp16_bits = 16
        
        weight_reduction = fp16_bits / self.qw['bits']
        kv_reduction = fp16_bits / self.qkv['bits']
        
        fp16_weight_bytes = model_params * 2
        fp16_kv_bytes = (2 * num_layers * num_heads * head_dim 
                         * context_length * 2)
        
        quant_weight_bytes = fp16_weight_bytes / weight_reduction
        quant_kv_bytes = fp16_kv_bytes / kv_reduction
        
        total_fp16 = fp16_weight_bytes + fp16_kv_bytes
        total_quant = quant_weight_bytes + quant_kv_bytes
        
        return {
            'fp16_total_gb': total_fp16 / 1e9,
            'quantized_total_gb': total_quant / 1e9,
            'reduction_factor': total_fp16 / total_quant,
            'freed_hbm_gb': (total_fp16 - total_quant) / 1e9,
        }


# Example: LLaMA-3.3-70B with 128K context
quantizer = PLENAQuantizer(
    qw_config={'type': 'MXINT', 'bits': 4},
    qact_config={'type': 'MXFP', 'bits': 4},
    qkv_config={'type': 'MiniFloat', 'bits': 4}
)

savings = quantizer.estimate_memory_reduction(
    model_params=70e9,
    context_length=128_000,
    num_layers=80,
    num_heads=64,
    head_dim=128
)
# Freed HBM enables larger batch sizes → higher throughput
```

### Pathway 3: Native FlashAttention Support

PLENA implements FlashAttention at the hardware level through three mechanisms:

```
Native FlashAttention Architecture:
┌─────────────────────────────────────────────────────────────────┐
│                                                                     │
│  Challenge 1: Online Softmax Reductions                             │
│  ─────────────────────────────────────                               │
│  Solution: Tightly coupled Vector + Scalar units                    │
│  • Vector unit width configurable to match FlashAttention tiles     │
│  • Implements required reductions and elementwise operations        │
│                                                                     │
│  Challenge 2: Transposed K Matrix Access                            │
│  ─────────────────────────────────────                               │
│  Solution: Matrix SRAM with dual-read layout                        │
│  • Read in standard OR transposed order — no data movement          │
│  • Multiple sub-SRAMs with lightweight address remapping            │
│  • Avoids costly on-the-fly transpose of K^T                       │
│  • Avoids storing K^T separately in HBM                            │
│                                                                     │
│  Challenge 3: Fine-Grained Scheduling                              │
│  ─────────────────────────────────────                               │
│  Solution: Custom ISA with tile-level control                        │
│  • Persistent, tile-by-tile scheduling of fused attention           │
│  • Each FlashAttention operation controlled individually             │
│  • Composable instructions enable flexible attention variants       │
│                                                                     │
│  FlashAttention Data Flow in PLENA:                                  │
│  ┌──────────┐    ┌──────────────┐    ┌──────────────┐              │
│  │ Q tiles  │───►│ Matrix Unit  │───►│ Softmax      │              │
│  │ (Vector  │    │ (Q × K^T)    │    │ (Vector Unit)│              │
│  │  SRAM)   │    │              │    │              │              │
│  └──────────┘    └──────────────┘    └──────┬───────┘              │
│                                              │                       │
│  ┌──────────┐    ┌──────────────┐    ┌──────▼───────┐              │
│  │ V tiles  │───►│ Matrix Unit  │◄───│ Attn weights │              │
│  │ (Matrix  │    │ (Attn × V)   │    │              │              │
│  │  SRAM)   │    │              │    │              │              │
│  └──────────┘    └──────────────┘    └──────────────┘              │
│                         │                                            │
│                         ▼                                            │
│                  ┌──────────────┐                                    │
│                  │ Output tiles │                                    │
│                  │ (on-chip)    │                                    │
│                  └──────────────┘                                    │
│                                                                     │
│  Key: Full attention matrix never materialized in HBM               │
│       All intermediate results stay on-chip                          │
└─────────────────────────────────────────────────────────────────────┘
```

## PLENA Hardware Architecture

```
PLENA System Architecture (7nm, 1 GHz):
┌─────────────────────────────────────────────────────────────────┐
│                         PLENA Accelerator                        │
│                                                                     │
│  ┌──────────────────── Compute Units ──────────────────────┐      │
│  │                                                           │      │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  │      │
│  │  │ MATRIX UNIT  │  │ VECTOR UNIT  │  │ SCALAR UNIT  │  │      │
│  │  │              │  │              │  │              │  │      │
│  │  │ Flattened    │  │ Elementwise  │  │ Integer      │  │      │
│  │  │ Systolic     │  │ Operations   │  │ Register     │  │      │
│  │  │ Array        │  │ Reductions   │  │ File         │  │      │
│  │  │ (8 × 512)    │  │ Softmax      │  │ Address      │  │      │
│  │  │              │  │ Activation   │  │ Manipulation │  │      │
│  │  │ Multi-type:  │  │ LayerNorm    │  │ Control Flow │  │      │
│  │  │ MXINT/MXFP/  │  │              │  │              │  │      │
│  │  │ MiniFloat    │  │ Configurable │  │              │  │      │
│  │  │              │  │ width        │  │              │  │      │
│  │  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘  │      │
│  │         │                  │                  │          │      │
│  └─────────┼──────────────────┼──────────────────┼──────────┘      │
│            │                  │                  │                   │
│  ┌─────────┼──────────────────┼──────────────────┼──────────┐      │
│  │         ▼                  ▼                  ▼          │      │
│  │  ┌──────────────┐  ┌──────────────┐                     │      │
│  │  │ MATRIX SRAM  │  │ VECTOR SRAM  │   Memory System     │      │
│  │  │              │  │              │                     │      │
│  │  │ Weights      │  │ Activations  │                     │      │
│  │  │ KV Tensors   │  │ Scratchpad   │                     │      │
│  │  │              │  │ (no HBM      │                     │      │
│  │  │ Dual-read:   │  │  writeback)  │                     │      │
│  │  │ Standard +   │  │              │                     │      │
│  │  │ Transposed   │  │              │                     │      │
│  │  │ (address     │  │              │                     │      │
│  │  │  remapping)  │  │              │                     │      │
│  │  └──────┬───────┘  └──────┬───────┘                     │      │
│  │         │                  │                              │      │
│  │  ┌──────▼──────────────────▼──────┐                     │      │
│  │  │     HBM LOAD UNITS             │                     │      │
│  │  │  • Background fetching          │                     │      │
│  │  │  • Streaming to SRAMs           │                     │      │
│  │  │  • Instruction-controlled       │                     │      │
│  │  └──────┬─────────────────────────┘                     │      │
│  └─────────┼────────────────────────────────────────────────┘      │
│            │                                                         │
│  ┌─────────▼────────────────────────────────────────────────┐      │
│  │              HBM (High Bandwidth Memory)                  │      │
│  │  ┌────────┐ ┌────────┐ ┌────────────┐ ┌────────────┐   │      │
│  │  │ Model  │ │ KV     │ │ Activation │ │ Batch      │   │      │
│  │  │ Weights│ │ Cache  │ │ Buffers    │ │ Data       │   │      │
│  │  └────────┘ └────────┘ └────────────┘ └────────────┘   │      │
│  │  144 GB capacity, 512 GB/s bandwidth                     │      │
│  └──────────────────────────────────────────────────────────┘      │
│                                                                     │
│  ┌────────────────────────────────────────────────────────┐        │
│  │              Instruction Buffer                         │        │
│  │  • 32-bit instructions from CPU via PCIe                │        │
│  │  • Instruction-level pipelining                         │        │
│  └────────────────────────────────────────────────────────┘        │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

### Matrix SRAM: Transpose Without Data Movement

A key innovation in PLENA's memory system is the Matrix SRAM's ability to serve data in both standard and transposed layouts without additional data movement:

```
Matrix SRAM Dual-Read Architecture:
┌─────────────────────────────────────────────────────────────────┐
│                                                                     │
│  Physical Storage (Sub-SRAMs):                                      │
│  ┌────┐ ┌────┐ ┌────┐ ┌────┐                                     │
│  │S0  │ │S1  │ │S2  │ │S3  │  ... (banked across sub-SRAMs)       │
│  │    │ │    │ │    │ │    │                                        │
│  │ a00│ │ a01│ │ a02│ │ a03│  ← Row 0                              │
│  │ a10│ │ a11│ │ a12│ │ a13│  ← Row 1                              │
│  │ a20│ │ a21│ │ a22│ │ a23│  ← Row 2                              │
│  └────┘ └────┘ └────┘ └────┘                                       │
│                                                                     │
│  Standard Read (row-major):                                         │
│  → Read S0[0], S1[0], S2[0], S3[0] → [a00, a01, a02, a03]        │
│                                                                     │
│  Transposed Read (column-major, via address remapping):             │
│  → Read S0[0], S0[1], S0[2] → [a00, a10, a20]                     │
│                                                                     │
│  Cost: Lightweight address remapping logic only                     │
│  Benefit: K^T access for FlashAttention with zero overhead          │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

## PLENA Software Stack

```
PLENA Software Toolchain:
┌─────────────────────────────────────────────────────────────────┐
│                                                                     │
│  ┌──────────────────────────────────────────────┐                  │
│  │              PyTorch Model                     │                  │
│  │  (LLaMA, GPT-OSS, any Transformer)             │                  │
│  └──────────────────┬───────────────────────────┘                  │
│                     │ Export                                         │
│                     ▼                                                │
│  ┌──────────────────────────────────────────────┐                  │
│  │              ONNX Format                       │                  │
│  │  • Standard graph optimizations                │                  │
│  │  • Constant folding                            │                  │
│  └──────────────────┬───────────────────────────┘                  │
│                     │ Pattern Matching                               │
│                     ▼                                                │
│  ┌──────────────────────────────────────────────┐                  │
│  │           PLENA Custom IR                      │                  │
│  │  Primitives: GEMM, Quantize, Dequantize,       │                  │
│  │              FlashAttention, Softmax, etc.      │                  │
│  └──────────────────┬───────────────────────────┘                  │
│                     │ Scheduling Search                              │
│                     ▼                                                │
│  ┌──────────────────────────────────────────────┐                  │
│  │         Schedule Optimizer                     │                  │
│  │  • Operator fusion                             │                  │
│  │  • Tiling configurations                       │                  │
│  │  • Memory placement                            │                  │
│  │  • Loop transformations                        │                  │
│  │  • Memory footprint validation                 │                  │
│  │  • Roofline-based performance model            │                  │
│  │  • Top-K schedule selection                    │                  │
│  └──────────────────┬───────────────────────────┘                  │
│                     │ Code Generation                               │
│                     ▼                                                │
│  ┌──────────────────────────────────────────────┐                  │
│  │          PLENA_ISA Assembly                    │                  │
│  │  Instruction classes:                          │                  │
│  │  • Matrix: GEMM operations                     │                  │
│  │  • Vector: Reductions, activations             │                  │
│  │  • Scalar: Address computation, control        │                  │
│  │  • Memory: Load/store to SRAMs                 │                  │
│  │  • Control: Branching, synchronization         │                  │
│  │                                                │                  │
│  │  32 bits per instruction                       │                  │
│  └──────────────────┬───────────────────────────┘                  │
│                     │                                                │
│                     ▼                                                │
│  ┌──────────────────────────────────────────────┐                  │
│  │  Cycle-Emulated Simulator (Rust)              │                  │
│  │  • Event-driven simulation                     │                  │
│  │  • HBM timing via Ramulator                    │                  │
│  │  • Cycle-accurate performance estimates        │                  │
│  └──────────────────────────────────────────────┘                  │
│                                                                     │
│  ┌──────────────────────────────────────────────┐                  │
│  │  Design Space Exploration (DSE)               │                  │
│  │  • Automated multi-objective Bayesian opt      │                  │
│  │  • Explores hardware params + quantization     │                  │
│  │  • Accuracy-aware: validates model quality     │                  │
│  │  • Pareto frontier discovery                   │                  │
│  └──────────────────────────────────────────────┘                  │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

### PLENA_ISA Design

```python
class PLENA_ISA:
    """
    Simplified representation of PLENA's instruction set architecture.
    
    Five instruction classes decouple responsibilities and
    allow flexible mixing across computation types.
    """
    
    INSTRUCTION_WIDTH = 32  # bits
    
    INSTRUCTION_CLASSES = {
        'MATRIX': [
            'MAT_GEMM',           # General matrix multiply
            'MAT_GEMM_QUANT',     # Quantized GEMM (mixed types)
            'MAT_FLASH_QK',       # FlashAttention Q×K^T
            'MAT_FLASH_AV',       # FlashAttention Attn×V
        ],
        'VECTOR': [
            'VEC_SOFTMAX',        # Online softmax reduction
            'VEC_LAYERNORM',      # Layer normalization
            'VEC_ACTIVATION',     # Activation functions
            'VEC_ELEMENTWISE',    # Elementwise operations
        ],
        'SCALAR': [
            'SCL_ADD',            # Address arithmetic
            'SCL_MUL',            # Index computation
            'SCL_LOAD_IMM',       # Load immediate
        ],
        'MEMORY': [
            'MEM_LOAD_MAT',       # Load to Matrix SRAM
            'MEM_LOAD_VEC',       # Load to Vector SRAM
            'MEM_STORE_HBM',      # Write back to HBM
            'MEM_LOAD_TRANSPOSE', # Load with transpose flag
        ],
        'CONTROL': [
            'CTL_BRANCH',         # Conditional branch
            'CTL_SYNC',           # Barrier synchronization
            'CTL_LOOP',           # Hardware loop
        ],
    }
    
    def tile_schedule_flash_attention(self, q_tiles, k_tiles, v_tiles):
        """
        Generate PLENA_ISA instruction sequence for FlashAttention.
        
        Persistent tile-by-tile scheduling keeps all intermediates
        on-chip, avoiding HBM materialization of attention matrix.
        """
        instructions = []
        
        for q_tile in q_tiles:
            instructions.append(('MEM_LOAD_VEC', q_tile))
            
            running_max = float('-inf')
            running_sum = 0
            
            for k_tile, v_tile in zip(k_tiles, v_tiles):
                instructions.append(('MEM_LOAD_MAT', k_tile))
                instructions.append(('MAT_FLASH_QK',))
                instructions.append(('VEC_SOFTMAX',))
                instructions.append(('MEM_LOAD_MAT', v_tile))
                instructions.append(('MAT_FLASH_AV',))
            
            instructions.append(('MEM_STORE_HBM', 'output_tile'))
        
        return instructions
```

## Benchmark Results

### System-Level Performance Comparison

All comparisons use identical HBM settings (144 GB capacity, 512 GB/s bandwidth) and equivalent multiplier counts, synthesized in 7nm technology at 1 GHz.

| Model | Config | A100 TPS | TPU v6e TPS | PLENA TPS | vs. A100 | vs. TPU v6e |
|-------|--------|----------|-------------|-----------|----------|-------------|
| Llama 3.2-1B | 128K ctx | Baseline | Baseline | Higher | Up to 2.24× | Up to 3.85× |
| Llama-3-8B | 128K ctx | Baseline | Baseline | Higher | — | — |
| Llama-3.3-70B | 128K ctx | Baseline | Baseline | Higher | — | — |
| GPT-OSS (MoE) | Long ctx | Baseline | Baseline | Higher | — | — |

### Utilization Comparison (LLaMA-3.3-70B, W4A4KV4)

| Array Shape | Workload | Utilization | vs. PLENA |
|-------------|----------|-------------|-----------|
| 64×64 (Square) | Standard (1K prompt, 128 gen) | High | ~1× |
| 64×64 (Square) | Agentic (5.6K prompt, 8K gen) | Low | PLENA is **8.5×** higher |
| **8×512 (PLENA)** | Standard (1K prompt, 128 gen) | High | — |
| **8×512 (PLENA)** | **Agentic (5.6K prompt, 8K gen)** | **High** | **Baseline** |

### Model Architecture Support

| Architecture Feature | Supported | Models |
|---------------------|-----------|--------|
| GQA (Grouped Query Attention) | Yes | LLaMA-3.x series |
| MHA (Multi-Head Attention) | Yes | Standard transformers |
| MLA (Multi-Latent Attention) | Yes | DeepSeek-style models |
| Dense | Yes | LLaMA, GPT |
| MoE (Mixture of Experts) | Yes | GPT-OSS |

### Implementation Details

| Parameter | Value |
|-----------|-------|
| Technology Node | 7nm (OpenROAD predictive PDK) |
| Clock Frequency | 1 GHz |
| Array Configuration | 8 × 512 (4,096 PEs) |
| HBM Capacity | 144 GB |
| HBM Bandwidth | 512 GB/s |
| Instruction Width | 32 bits |
| Simulator | Rust-based, event-driven, HBM via Ramulator |
| RTL Language | SystemVerilog |

## Connection to the Complexity Trap

PLENA addresses the hardware layer of the same problem that the Complexity Trap research (Lindenbauer et al., 2025) tackles at the software layer. The two approaches are complementary:

### Complementary Approaches at Different Layers

```
The Full Stack for Efficient Agentic Inference:
┌─────────────────────────────────────────────────────────────────┐
│                                                                     │
│  Layer 4: CONTEXT MANAGEMENT (Complexity Trap)                      │
│  ─────────────────────────────────────────────                      │
│  • Observation masking reduces token count by ~50%                  │
│  • Fewer tokens → fewer GEMM operations → less HBM traffic         │
│  • Directly alleviates BOTH memory walls                            │
│                                                                     │
│  Layer 3: ALGORITHMIC (FlashAttention)                              │
│  ─────────────────────────────────────────────                      │
│  • Fused attention avoids materializing N×N matrix                  │
│  • Reduces HBM reads/writes during attention                        │
│  • PLENA implements this natively in hardware                       │
│                                                                     │
│  Layer 2: NUMERICAL (Quantization)                                  │
│  ─────────────────────────────────────────────                      │
│  • Asymmetric quantization reduces per-token memory                 │
│  • 4-bit KV cache → 4× more context in same HBM                    │
│  • PLENA supports mixed types natively                              │
│                                                                     │
│  Layer 1: HARDWARE (PLENA Architecture)                             │
│  ─────────────────────────────────────────────                      │
│  • Flattened systolic array matches agentic GEMM shapes            │
│  • 8.5× utilization improvement for long-context workloads         │
│  • Dual-read Matrix SRAM eliminates transpose overhead              │
│                                                                     │
│  Combined Effect:                                                    │
│  Masking (2× fewer tokens) × Quantization (4× smaller) ×           │
│  Architecture (8.5× utilization) = Massive efficiency gain          │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

### Specific Synergies

| Complexity Trap Finding | PLENA Amplification |
|------------------------|---------------------|
| Observation masking reduces tokens by ~50% | 50% fewer tokens means 50% less KV cache → pushes back capacity wall |
| Unmanaged contexts double costs | Hardware efficiency multiplies software savings |
| Hybrid approach achieves 7-11% further cost reduction | Quantized context storage makes hybrids even more economical |
| Trajectory elongation wastes compute | PLENA's higher utilization reduces per-token cost of elongation |
| Context management is essential for viability | Hardware acceleration makes managed contexts deployable at scale |

### Multiplied Efficiency Gains

```python
def combined_efficiency_gain():
    """
    Estimate combined efficiency from software + hardware optimization.
    
    Software layer (Complexity Trap):
      Observation masking → ~50% token reduction
      Hybrid approach → additional 7-11% cost reduction
    
    Hardware layer (PLENA):
      Flattened array → 8.5× utilization
      Quantization → ~4× memory reduction
      FlashAttention → eliminates O(N^2) materialization
    """
    # Software savings (per the Complexity Trap)
    masking_reduction = 0.50        # 50% fewer tokens
    hybrid_additional = 0.10        # ~10% further reduction
    software_factor = (1 - masking_reduction) * (1 - hybrid_additional)
    # software_factor ≈ 0.45 (55% total software reduction)
    
    # Hardware savings (per PLENA)
    utilization_gain = 8.5          # vs. square systolic array
    quantization_gain = 4.0         # W4A4KV4 vs. FP16
    throughput_vs_a100 = 2.24       # under same resource constraints
    
    # Combined: fewer tokens processed on more efficient hardware
    combined_tokens = software_factor  # 45% of original token volume
    combined_throughput = throughput_vs_a100  # 2.24× faster per token
    
    effective_speedup = (1 / combined_tokens) * combined_throughput
    
    return {
        'software_token_reduction': f'{(1 - software_factor) * 100:.0f}%',
        'hardware_throughput_gain': f'{combined_throughput:.2f}×',
        'combined_effective_speedup': f'{effective_speedup:.1f}×',
    }

# Result: ~5× effective speedup from combining both approaches
```

### Validating the Complexity Trap Thesis

PLENA provides hardware-level validation of a core Complexity Trap insight: **efficiency gains compound across the stack**. The Complexity Trap demonstrated that simple context management (observation masking) delivers outsized returns at the software level. PLENA shows that hardware-level optimizations can multiply those returns:

1. **Simple solutions enable hardware optimization** — Observation masking produces predictable, structured access patterns that hardware can optimize for. Complex summarization produces variable-length, unpredictable outputs that are harder to accelerate.

2. **Efficiency is a first-class concern at every layer** — Just as the Complexity Trap argues against over-engineering context management, PLENA argues against over-engineering hardware for the wrong workload shape. Both advocate matching the solution to the actual problem.

3. **The economic argument strengthens** — At $0.03 per instance savings from software optimization alone (Complexity Trap), adding 2.24× hardware throughput improvement makes the combined economics compelling for production deployment.

## Future Directions

From the PLENA paper:

1. **Multi-core flattened systolic array** — Exploit parallelism with multiple PLENA cores
2. **Enhanced compiler scheduling** — Finer-grained control over execution
3. **GPU-PLENA heterogeneous systems** — Combine GPU flexibility with PLENA efficiency
4. **FlashAttention GEMM optimization** — Further improve utilization during attention

### Integration with Software Context Management

| Direction | Description | Expected Impact |
|-----------|-------------|-----------------|
| Context-aware scheduling | Compiler adapts to masked vs. full observations | Optimized memory access patterns |
| Quantization-aware masking | Choose masking threshold based on quantization level | Balanced accuracy-efficiency tradeoff |
| Hardware-guided context budgets | PLENA capacity informs software context limits | Optimal context window utilization |
| Streaming context processing | Process trajectory in tiles as generated | Lower latency for agent responses |

## References

```bibtex
@article{wu2025plena,
  title={Combating the Memory Walls: Optimization Pathways for Long-Context Agentic LLM Inference},
  author={Wu, Haoran and others},
  journal={arXiv:2509.09505},
  year={2025}
}

@inproceedings{lindenbauer2025complexity,
  title={The Complexity Trap: Simple Observation Masking Is as Efficient as LLM Summarization for Agent Context Management},
  author={Lindenbauer, Tobias and others},
  booktitle={NeurIPS 2025 Workshop: Deep Learning for Code in the Agentic Era},
  year={2025}
}

@article{dao2023flashattention2,
  title={FlashAttention-2: Faster Attention with Better Parallelism and Work Partitioning},
  author={Dao, Tri},
  journal={arXiv:2307.08691},
  year={2023}
}
```

## Next Steps

- **[The Problem](../architecture/02-the-problem.md)** — Context bloat that PLENA helps address at hardware level
- **[Observation Masking](../strategies/01-observation-masking.md)** — Software strategy that complements PLENA
- **[Hybrid Approach](../strategies/03-hybrid-approach.md)** — Combined software strategy for maximum efficiency
- **[Future Work](../challenges/02-future-work.md)** — Hardware-aware optimization as open research direction
