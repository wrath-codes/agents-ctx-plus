# Semantic Triggers for Context Management

## Overview

Current context management strategies (observation masking, LLM summarization, hybrid) rely on **turn-count-based triggers** — compression occurs after fixed numbers of turns (M=10, N=21, N=43). This approach, while simple and effective, is semantically blind — it cannot distinguish between meaningful task boundaries and arbitrary turn accumulation.

**Semantic triggers** offer a more intelligent approach: compress context based on what the agent is doing rather than how many turns have passed. This enables more natural, effective compression that preserves task-relevant information while removing redundancy.

---

## The Problem with Turn-Count Triggers

```
┌─────────────────────────────────────────────────────────────────────────────┐
│              TURN-COUNT TRIGGERS (Semantically Blind)                      │
│                                                                             │
│  Scenario 1: Rapid Progress (Simple Task)                                   │
│  ───────────────────────────────────────                                   │
│  Turns 1-5: Read file → Edit file → Run test → Success                     │
│  Trigger at M=10: No masking yet                                            │
│  Result: Wasted tokens on short trajectory                                  │
│                                                                             │
│  Scenario 2: Long Subtask (Complex Investigation)                          │
│  Turns 1-15: Deep debugging single issue                                   │
│  Trigger at M=10: Masks turn 1 at turn 11                                  │
│  Result: Loses context from early investigation                            │
│                                                                             │
│  Scenario 3: Context Switch (New File)                                     │
│  Turns 1-8: Work on auth.py                                                │
│  Turns 9-12: Switch to utils.py                                            │
│  Trigger at M=10: Masks turns 1-2 at turn 11                               │
│  Result: Could have compressed auth.py context at turn 9                   │
│                                                                             │
│  ═══════════════════════════════════════════════════════════════════════  │
│  Problem: Fixed thresholds don't adapt to task structure                     │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Semantic Trigger Types

### 1. Subtask Completion Detection

**Concept**: Trigger compression when the agent completes a semantically coherent unit of work.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│              SUBTASK COMPLETION DETECTION                                   │
│                                                                             │
│  Task: "Fix authentication bug and update documentation"                   │
│                                                                             │
│  Subgoal 1: Fix authentication bug                                         │
│  ─────────────────────────────────                                           │
│  Turn 1: Read auth.py → Observation: 500 tokens                            │
│  Turn 2: Run test → Observation: 200 tokens (error trace)                    │
│  Turn 3: Edit auth.py → Observation: 100 tokens                            │
│  Turn 4: Run test → Observation: 50 tokens (success)                       │
│  ─────────────────────────────────                                           │
│  ★ SUBTASK COMPLETE: Authentication bug fixed                              │
│  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━  │
│  TRIGGER: Compress turns 1-3 to summary                                    │
│  RETAIN: Turn 4 outcome (success signal)                                     │
│                                                                             │
│  Subgoal 2: Update documentation                                             │
│  ─────────────────────────────────                                           │
│  Turn 5: Read README.md → Observation: 300 tokens                            │
│  Turn 6: Edit README.md → Observation: 100 tokens                          │
│  Turn 7: Verify changes → Observation: 50 tokens                           │
│  ─────────────────────────────────                                           │
│  ★ SUBTASK COMPLETE: Documentation updated                                   │
│  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━  │
│  TRIGGER: Compress turns 5-6 to summary                                    │
│                                                                             │
│  Result: Natural compression at meaningful boundaries                        │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Detection Signals**:

| Signal | Description | Confidence |
|--------|-------------|------------|
| Test success | Unit test passes after edits | High |
| Explicit completion | Agent states "this is complete" | High |
| File save | Multiple consecutive successful edits | Medium |
| Semantic marker | Agent says "now I'll work on X" | Medium |
| Tool pattern | read → edit → test → success sequence | Medium |
| Inactivity | No file changes for N turns | Low |

### 2. Intent-Based Triggering

**Concept**: Detect when the agent's intent shifts, indicating a natural compression point.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    INTENT SHIFT DETECTION                                  │
│                                                                             │
│  Intent Classification:                                                     │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ Intent Category       │ Examples                                    │   │
│  │───────────────────────│─────────────────────────────────────────────│   │
│  │ EXPLORE               │ "Let me understand the codebase"           │   │
│  │ DEBUG                 │ "I need to find why the test fails"        │   │
│  │ IMPLEMENT             │ "I'll add the missing feature"             │   │
│  │ VERIFY                │ "Let me run the tests to confirm"          │   │
│  │ DOCUMENT              │ "I should update the README"             │   │
│  │ REFACTOR              │ "This code needs cleaning up"              │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  Intent Transition Triggers:                                                │
│                                                                             │
│  EXPLORE → IMPLEMENT: Compress exploration phase                           │
│    "I now understand the structure, I'll implement the fix"                │
│    → Summary: "Codebase analyzed: auth.py handles tokens, bug in line 45" │
│                                                                             │
│  DEBUG → VERIFY: Compress debugging attempts                               │
│    "I found the issue, let me verify the fix works"                        │
│    → Summary: "Debugged: KeyError caused by missing null check"            │
│                                                                             │
│  IMPLEMENT → DOCUMENT: Compress implementation details                     │
│    "Feature implemented, now I'll update docs"                             │
│    → Summary: "Added OAuth2 flow to auth.py, 3 new methods"                │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Intent Classification Algorithm**:

```python
class IntentBasedTrigger:
    """
    Trigger context compression based on intent transitions.
    """
    
    INTENTS = ['EXPLORE', 'DEBUG', 'IMPLEMENT', 'VERIFY', 'DOCUMENT', 'REFACTOR']
    
    # Compression-worthy transitions
    COMPRESSION_TRANSITIONS = [
        ('EXPLORE', 'IMPLEMENT'),   # Done exploring, start fixing
        ('DEBUG', 'VERIFY'),        # Done debugging, verify fix
        ('IMPLEMENT', 'DOCUMENT'),  # Done coding, start docs
        ('IMPLEMENT', 'VERIFY'),    # Done coding, run tests
    ]
    
    def __init__(self, classifier_model):
        self.classifier = classifier_model
        self.intent_history = []
        
    def classify_intent(self, reasoning: str, action: dict) -> str:
        """Classify agent's current intent from reasoning and action."""
        prompt = f"""
        Classify the agent's intent based on reasoning and action.
        
        Reasoning: {reasoning}
        Action: {action['type']} - {action.get('details', '')}
        
        Choose from: EXPLORE, DEBUG, IMPLEMENT, VERIFY, DOCUMENT, REFACTOR
        
        Consider:
        - EXPLORE: Understanding, reading, searching without specific fix
        - DEBUG: Investigating failures, tracing errors
        - IMPLEMENT: Writing code, adding features, making changes
        - VERIFY: Testing, confirming, validating
        - DOCUMENT: Writing comments, updating README
        - REFACTOR: Cleaning code, restructuring without functional change
        
        Respond with just the intent label.
        """
        
        return self.classifier.generate(prompt).strip()
    
    def should_compress(self, current_intent: str, 
                        trajectory: list) -> tuple[bool, str]:
        """
        Determine if compression should trigger based on intent.
        
        Returns: (should_compress, compression_scope)
        """
        if not self.intent_history:
            self.intent_history.append(current_intent)
            return False, ""
        
        previous_intent = self.intent_history[-1]
        
        # Check for compression-worthy transition
        if (previous_intent, current_intent) in self.COMPRESSION_TRANSITIONS:
            # Find where previous intent started
            transition_point = self._find_intent_start(previous_intent)
            
            return True, f"turns_{transition_point}_{len(trajectory)-1}"
        
        self.intent_history.append(current_intent)
        return False, ""
    
    def _find_intent_start(self, intent: str) -> int:
        """Find where the given intent started in history."""
        for i, hist_intent in enumerate(reversed(self.intent_history)):
            if hist_intent != intent:
                return len(self.intent_history) - i
        return 0
```

### 3. Semantic Boundary Detection

**Concept**: Detect natural boundaries in the task space (file boundaries, module boundaries, test boundaries).

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                 SEMANTIC BOUNDARY DETECTION                                │
│                                                                             │
│  Boundary Types:                                                            │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ 1. FILE BOUNDARIES                                                   │   │
│  │    Trigger: Agent switches to different file                         │   │
│  │    Compression: Summarize work on previous file                      │   │
│  │                                                                     │   │
│  │    Example:                                                          │   │
│  │    Turns 1-8: Work on auth.py                                        │   │
│  │    Turn 9: Action: read_file("utils.py")                             │   │
│  │    ★ FILE BOUNDARY: auth.py → utils.py                               │   │
│  │    → Compress auth.py work to summary                                │   │
│  │                                                                     │   │
│  │ 2. MODULE BOUNDARIES                                                 │   │
│  │    Trigger: Cross-module imports or calls                            │   │
│  │    Compression: Summarize module-level context                         │   │
│  │                                                                     │   │
│  │    Example:                                                          │   │
│  │    Working in src/auth/                                              │   │
│  │    Action: read_file("../database/models.py")                          │   │
│  │    ★ MODULE BOUNDARY: auth → database                                │   │
│  │    → Compress auth module context                                    │   │
│  │                                                                     │   │
│  │ 3. TEST BOUNDARIES                                                   │   │
│  │    Trigger: Test execution boundaries                                │   │
│  │    Compression: Summarize between test runs                            │   │
│  │                                                                     │   │
│  │    Example:                                                          │   │
│  │    Turns 1-5: Debugging test failure                                 │   │
│  │    Turn 6: Test passes                                               │   │
│  │    Turns 7-10: Working on different test                             │   │
│  │    ★ TEST BOUNDARY: Completed test case                                │   │
│  │    → Compress debugging for passed test                                │   │
│  │                                                                     │   │
│  │ 4. API BOUNDARIES                                                    │   │
│  │    Trigger: External API call patterns                                 │   │
│  │    Compression: Summarize API interaction                              │   │
│  │                                                                     │   │
│  │    Example:                                                          │   │
│  │    Turns 1-4: API exploration (docs, endpoints)                        │   │
│  │    Turn 5: First actual API call                                       │   │
│  │    ★ API BOUNDARY: Exploration → Usage                                 │   │
│  │    → Compress exploration, keep API usage pattern                      │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Boundary Detection Algorithm**:

```python
class SemanticBoundaryDetector:
    """
    Detect semantic boundaries for natural compression points.
    """
    
    def __init__(self):
        self.current_file = None
        self.current_module = None
        self.current_test = None
        self.boundary_history = []
        
    def detect_boundary(self, action: dict, observation: str) -> dict:
        """
        Detect if this action crosses a semantic boundary.
        
        Returns boundary info or None if no boundary crossed.
        """
        boundary = None
        
        # File boundary detection
        if action['type'] == 'read_file':
            new_file = action['path']
            if self.current_file and new_file != self.current_file:
                boundary = {
                    'type': 'FILE',
                    'from': self.current_file,
                    'to': new_file,
                    'turn': len(self.boundary_history)
                }
            self.current_file = new_file
        
        # Module boundary detection
        new_module = self._extract_module(new_file if action['type'] == 'read_file' 
                                         else self.current_file)
        if self.current_module and new_module != self.current_module:
            boundary = {
                'type': 'MODULE',
                'from': self.current_module,
                'to': new_module,
                'turn': len(self.boundary_history)
            }
        self.current_module = new_module
        
        # Test boundary detection
        if action['type'] == 'run_test':
            test_name = action.get('test_name', 'unknown')
            
            # Check if test completed (success or failure)
            if 'PASSED' in observation or 'FAILED' in observation:
                if self.current_test and test_name != self.current_test:
                    boundary = {
                        'type': 'TEST',
                        'test': self.current_test,
                        'result': 'PASSED' if 'PASSED' in observation else 'FAILED',
                        'turn': len(self.boundary_history)
                    }
                self.current_test = test_name
        
        if boundary:
            self.boundary_history.append(boundary)
        
        return boundary
    
    def _extract_module(self, file_path: str) -> str:
        """Extract module name from file path."""
        if not file_path:
            return None
        parts = file_path.split('/')
        if len(parts) >= 2:
            return parts[0] if parts[0] else parts[1]
        return parts[0] if parts else None
    
    def get_compression_candidates(self, trajectory: list) -> list:
        """
        Get list of turns that can be compressed based on boundaries.
        
        Returns list of (start_turn, end_turn, boundary_type) tuples.
        """
        candidates = []
        
        for i, boundary in enumerate(self.boundary_history):
            if i == 0:
                start = 0
            else:
                start = self.boundary_history[i-1]['turn']
            
            end = boundary['turn']
            
            # Only compress if span is meaningful (>3 turns)
            if end - start > 3:
                candidates.append({
                    'start': start,
                    'end': end,
                    'type': boundary['type'],
                    'boundary': boundary
                })
        
        return candidates
```

### 4. Information Staleness Detection

**Concept**: Detect when information becomes stale and can be compressed.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│               INFORMATION STALENESS DETECTION                              │
│                                                                             │
│  Staleness Factors:                                                         │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ Factor          │ Weight │ Detection Method                         │   │
│  │─────────────────│────────│──────────────────────────────────────────│   │
│  │ Time (turns)    │ 0.3    │ Raw age in turns                         │   │
│  │ Reference count │ 0.4    │ How often accessed in recent turns     │   │
│  │ Relevance decay │ 0.3    │ Embedding similarity to current context│   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  Staleness Score Formula:                                                   │
│  S(t) = 0.3 × (age / max_age) + 0.4 × (1 - recent_refs / max_refs)          │
│         + 0.3 × (1 - embedding_similarity)                                  │
│                                                                             │
│  Compression Trigger: S(t) > 0.7                                            │
│                                                                             │
│  Example Calculation:                                                       │
│  ─────────────────────                                                      │
│  Turn 1: Read config.py (500 tokens)                                        │
│  At Turn 20:                                                                │
│    - Age factor: 0.3 × (19 / 50) = 0.114                                    │
│    - Reference factor: 0.4 × (1 - 0 / 10) = 0.4                           │
│    - Relevance factor: 0.3 × (1 - 0.2) = 0.24                              │
│    - Total staleness: 0.754 > 0.7 → TRIGGER COMPRESSION                       │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Hybrid Semantic-Turn System

The most practical approach combines semantic triggers with turn-count fallbacks:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│            HYBRID SEMANTIC-TURN TRIGGER SYSTEM                             │
│                                                                             │
│  Priority Order:                                                            │
│                                                                             │
│  1. SEMANTIC TRIGGERS (Highest Priority)                                    │
│     - Subtask completion detected                                            │
│     - Intent transition (EXPLORE→IMPLEMENT)                                  │
│     - File/module boundary crossed                                           │
│     → Immediate compression at natural boundary                              │
│                                                                             │
│  2. STALENESS TRIGGERS (Medium Priority)                                    │
│     - Information staleness score > 0.7                                      │
│     - No references for N turns                                              │
│     → Compress specific stale observations                                   │
│                                                                             │
│  3. TURN-COUNT FALLBACK (Lowest Priority)                                   │
│     - M turns reached with no semantic trigger                               │
│     - Force compression to prevent unbounded growth                          │
│     → Apply standard masking/summarization                                   │
│                                                                             │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  Decision Flow:                                                           │
│                                                                             │
│      ┌─────────────┐                                                        │
│      │ New Turn    │                                                        │
│      └──────┬──────┘                                                        │
│             ▼                                                                │
│      ┌─────────────┐                                                        │
│      │ Semantic    │                                                        │
│      │ Trigger?    │                                                        │
│      └──────┬──────┘                                                        │
│        YES /   \ NO                                                         │
│            ▼     ▼                                                           │
│    ┌──────────┐  ┌─────────────┐                                            │
│    │ Compress │  │ Stale Info? │                                            │
│    │ at       │  │ Score > 0.7 │                                            │
│    │ Boundary │  └──────┬──────┘                                            │
│    └──────────┘   YES /   \ NO                                               │
│                      ▼     ▼                                                  │
│              ┌────────┐  ┌─────────────┐                                      │
│              │Compress│  │ Turn Count > M? │                              │
│              │ Stale  │  └──────┬──────┘                                    │
│              └────────┘   YES /   \ NO                                       │
│                          ▼       ▼                                           │
│                  ┌────────┐  ┌──────────┐                                    │
│                  │ Apply  │  │ Continue │                                    │
│                  │ Masking│  │ (no action)│                                  │
│                  └────────┘  └──────────┘                                    │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Implementation

```python
class SemanticHybridTrigger:
    """
    Hybrid semantic-turn trigger system for context compression.
    
    Prioritizes semantic triggers, falls back to turn-count.
    """
    
    def __init__(self, llm_client, config: dict = None):
        self.llm = llm_client
        self.config = config or {
            'masking_window': 10,
            'staleness_threshold': 0.7,
            'enable_intent_detection': True,
            'enable_boundary_detection': True,
            'enable_staleness_detection': True
        }
        
        # Sub-detectors
        self.intent_detector = IntentBasedTrigger(llm_client)
        self.boundary_detector = SemanticBoundaryDetector()
        self.staleness_tracker = StalenessTracker()
        
        self.trajectory = []
        self.compression_points = []
        
    def process_turn(self, reasoning: str, action: dict, 
                     observation: str) -> dict:
        """
        Process new turn and determine if compression should trigger.
        
        Returns: Compression decision with metadata
        """
        turn_idx = len(self.trajectory)
        
        # Store turn
        self.trajectory.append({
            'turn': turn_idx,
            'reasoning': reasoning,
            'action': action,
            'observation': observation
        })
        
        decision = {
            'should_compress': False,
            'trigger_type': None,
            'compression_scope': None,
            'confidence': 0.0
        }
        
        # Priority 1: Semantic triggers
        if self.config['enable_intent_detection']:
            intent = self.intent_detector.classify_intent(reasoning, action)
            should_compress, scope = self.intent_detector.should_compress(
                intent, self.trajectory
            )
            if should_compress:
                decision.update({
                    'should_compress': True,
                    'trigger_type': 'INTENT_TRANSITION',
                    'compression_scope': scope,
                    'confidence': 0.85,
                    'details': {'intent': intent}
                })
                return decision
        
        if self.config['enable_boundary_detection']:
            boundary = self.boundary_detector.detect_boundary(action, observation)
            if boundary:
                candidates = self.boundary_detector.get_compression_candidates(
                    self.trajectory
                )
                if candidates:
                    candidate = candidates[-1]
                    decision.update({
                        'should_compress': True,
                        'trigger_type': f"BOUNDARY_{boundary['type']}",
                        'compression_scope': f"turns_{candidate['start']}_{candidate['end']}",
                        'confidence': 0.80,
                        'details': boundary
                    })
                    return decision
        
        # Priority 2: Staleness triggers
        if self.config['enable_staleness_detection']:
            stale_observations = self.staleness_tracker.get_stale_observations(
                self.trajectory,
                threshold=self.config['staleness_threshold']
            )
            if stale_observations:
                decision.update({
                    'should_compress': True,
                    'trigger_type': 'STALENESS',
                    'compression_scope': stale_observations,
                    'confidence': 0.70,
                    'details': {'count': len(stale_observations)}
                })
                return decision
        
        # Priority 3: Turn-count fallback
        if turn_idx > self.config['masking_window']:
            # Check if we're in masking phase
            recent_uncompressed = sum(
                1 for t in self.trajectory[-self.config['masking_window']:]
                if not t.get('compressed', False)
            )
            
            if recent_uncompressed >= self.config['masking_window']:
                decision.update({
                    'should_compress': True,
                    'trigger_type': 'TURN_COUNT_FALLBACK',
                    'compression_scope': f"turn_{turn_idx - self.config['masking_window']}",
                    'confidence': 0.50
                })
        
        return decision
    
    def apply_compression(self, decision: dict) -> list:
        """Apply compression based on trigger decision."""
        if not decision['should_compress']:
            return self.trajectory
        
        trigger = decision['trigger_type']
        scope = decision['compression_scope']
        
        if trigger == 'INTENT_TRANSITION':
            # Compress previous intent phase
            start, end = self._parse_scope(scope)
            summary = self._summarize_range(start, end)
            
            for i in range(start, end + 1):
                self.trajectory[i]['compressed'] = True
                if i == end:
                    self.trajectory[i]['summary'] = summary
                    
        elif trigger.startswith('BOUNDARY'):
            # Compress up to boundary
            start, end = self._parse_scope(scope)
            summary = self._summarize_range(start, end)
            
            for i in range(start, end + 1):
                self.trajectory[i]['compressed'] = True
                if i == end:
                    self.trajectory[i]['summary'] = summary
                    
        elif trigger == 'STALENESS':
            # Compress specific stale observations
            for obs_info in scope:
                turn_idx = obs_info['turn']
                self.trajectory[turn_idx]['observation'] = '[Stale observation omitted]'
                self.trajectory[turn_idx]['compressed'] = True
                
        elif trigger == 'TURN_COUNT_FALLBACK':
            # Apply standard masking
            turn_idx = int(scope.split('_')[1])
            self.trajectory[turn_idx]['observation'] = '[Observation omitted]'
            self.trajectory[turn_idx]['compressed'] = True
        
        self.compression_points.append({
            'turn': len(self.trajectory) - 1,
            'trigger': trigger,
            'scope': scope
        })
        
        return self.trajectory
    
    def _parse_scope(self, scope: str) -> tuple:
        """Parse 'turns_X_Y' into (X, Y)."""
        parts = scope.split('_')
        return int(parts[1]), int(parts[2])
    
    def _summarize_range(self, start: int, end: int) -> str:
        """Generate summary for turn range."""
        trajectory_text = self._format_trajectory_range(start, end)
        
        prompt = f"""
        Summarize the following agent trajectory concisely.
        Preserve key outcomes, decisions, and state changes.
        
        Trajectory (Turns {start}-{end}):
        {trajectory_text}
        
        Provide a 1-2 sentence summary of what was accomplished.
        """
        
        return self.llm.generate(prompt)
```

---

## Expected Benefits

| Metric | Turn-Count Only | With Semantic Triggers | Improvement |
|--------|-----------------|----------------------:|-------------|
| Compression precision | Low (arbitrary turns) | High (meaningful boundaries) | **+40%** |
| Information preservation | Medium | High | **+25%** |
| Token efficiency | Good | Better | **+15%** |
| Solve rate impact | -1.0 pp (potential) | +0.5 pp (potential) | **+1.5 pp** |
| Implementation complexity | Simple | Moderate | — |

---

## 2025 Research Validation

Recent research validates the importance of semantic triggers:

1. **HiAgent (ACL 2025)**: Demonstrates subgoal-based detection achieves 35% context reduction
2. **ACE (ICLR 2026)**: Modular curation validates execution feedback-driven triggers
3. **CASK (AAMAS 2025)**: Saliency-based triggers provide principled staleness detection

---

## Connection to Complexity Trap

Semantic triggers represent the next evolution beyond the hybrid approach:
- **Replace fixed thresholds** (M, N) with adaptive semantic detection
- **Preserve natural task structure** instead of arbitrary turn boundaries
- **Enable more aggressive compression** when semantically appropriate

---

*Next: [Advanced Strategies](04-advanced-strategies.md) | [Trajectory Evaluation](../experiments/05-trajectory-evaluation.md)*
