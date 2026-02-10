//! Status enums, entity types, relations, and actions for Zenith.
//!
//! All enums use `snake_case` serialization via `#[serde(rename_all = "snake_case")]`.
//! Status enums with state machines provide `allowed_next_states()` to enforce
//! valid transitions at the application layer.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;

// ---------------------------------------------------------------------------
// Confidence
// ---------------------------------------------------------------------------

/// Confidence level for findings and insights.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    High,
    Medium,
    Low,
}

impl Confidence {
    /// Return the string representation used in SQL storage.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::High => "high",
            Self::Medium => "medium",
            Self::Low => "low",
        }
    }
}

impl fmt::Display for Confidence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

// ---------------------------------------------------------------------------
// HypothesisStatus
// ---------------------------------------------------------------------------

/// Status of a hypothesis through its investigation lifecycle.
///
/// ```text
/// unverified → analyzing → confirmed
///                        → debunked
///                        → partially_confirmed
///                        → inconclusive
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HypothesisStatus {
    Unverified,
    Analyzing,
    Confirmed,
    Debunked,
    PartiallyConfirmed,
    Inconclusive,
}

impl HypothesisStatus {
    /// Valid next states from the current state.
    #[must_use]
    pub const fn allowed_next_states(self) -> &'static [Self] {
        match self {
            Self::Unverified => &[Self::Analyzing],
            Self::Analyzing => &[
                Self::Confirmed,
                Self::Debunked,
                Self::PartiallyConfirmed,
                Self::Inconclusive,
            ],
            Self::Confirmed | Self::Debunked | Self::PartiallyConfirmed | Self::Inconclusive => &[],
        }
    }

    /// Check whether transitioning to `next` is allowed.
    #[must_use]
    pub fn can_transition_to(self, next: Self) -> bool {
        self.allowed_next_states().contains(&next)
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unverified => "unverified",
            Self::Analyzing => "analyzing",
            Self::Confirmed => "confirmed",
            Self::Debunked => "debunked",
            Self::PartiallyConfirmed => "partially_confirmed",
            Self::Inconclusive => "inconclusive",
        }
    }
}

impl fmt::Display for HypothesisStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

// ---------------------------------------------------------------------------
// TaskStatus
// ---------------------------------------------------------------------------

/// Status of a task.
///
/// ```text
/// open → in_progress → done
///                    → blocked → in_progress (unblocked)
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Open,
    InProgress,
    Done,
    Blocked,
}

impl TaskStatus {
    #[must_use]
    #[allow(clippy::match_same_arms)]
    pub const fn allowed_next_states(self) -> &'static [Self] {
        match self {
            Self::Open => &[Self::InProgress],
            Self::InProgress => &[Self::Done, Self::Blocked],
            Self::Blocked => &[Self::InProgress],
            Self::Done => &[],
        }
    }

    #[must_use]
    pub fn can_transition_to(self, next: Self) -> bool {
        self.allowed_next_states().contains(&next)
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::InProgress => "in_progress",
            Self::Done => "done",
            Self::Blocked => "blocked",
        }
    }
}

impl fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

// ---------------------------------------------------------------------------
// IssueType
// ---------------------------------------------------------------------------

/// Type of an issue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum IssueType {
    Bug,
    Feature,
    Spike,
    Epic,
    Request,
}

impl IssueType {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Bug => "bug",
            Self::Feature => "feature",
            Self::Spike => "spike",
            Self::Epic => "epic",
            Self::Request => "request",
        }
    }
}

impl fmt::Display for IssueType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

// ---------------------------------------------------------------------------
// IssueStatus
// ---------------------------------------------------------------------------

/// Status of an issue.
///
/// ```text
/// open → in_progress → done
///                    → blocked → in_progress (unblocked)
///                    → abandoned
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum IssueStatus {
    Open,
    InProgress,
    Done,
    Blocked,
    Abandoned,
}

impl IssueStatus {
    #[must_use]
    #[allow(clippy::match_same_arms)]
    pub const fn allowed_next_states(self) -> &'static [Self] {
        match self {
            Self::Open => &[Self::InProgress],
            Self::InProgress => &[Self::Done, Self::Blocked, Self::Abandoned],
            Self::Blocked => &[Self::InProgress],
            Self::Done | Self::Abandoned => &[],
        }
    }

    #[must_use]
    pub fn can_transition_to(self, next: Self) -> bool {
        self.allowed_next_states().contains(&next)
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::InProgress => "in_progress",
            Self::Done => "done",
            Self::Blocked => "blocked",
            Self::Abandoned => "abandoned",
        }
    }
}

impl fmt::Display for IssueStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

// ---------------------------------------------------------------------------
// ResearchStatus
// ---------------------------------------------------------------------------

/// Status of a research item.
///
/// ```text
/// open → in_progress → resolved
///                    → abandoned
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ResearchStatus {
    Open,
    InProgress,
    Resolved,
    Abandoned,
}

impl ResearchStatus {
    #[must_use]
    pub const fn allowed_next_states(self) -> &'static [Self] {
        match self {
            Self::Open => &[Self::InProgress],
            Self::InProgress => &[Self::Resolved, Self::Abandoned],
            Self::Resolved | Self::Abandoned => &[],
        }
    }

    #[must_use]
    pub fn can_transition_to(self, next: Self) -> bool {
        self.allowed_next_states().contains(&next)
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::InProgress => "in_progress",
            Self::Resolved => "resolved",
            Self::Abandoned => "abandoned",
        }
    }
}

impl fmt::Display for ResearchStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

// ---------------------------------------------------------------------------
// SessionStatus
// ---------------------------------------------------------------------------

/// Status of a work session.
///
/// ```text
/// active → wrapped_up
///        → abandoned
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Active,
    WrappedUp,
    Abandoned,
}

impl SessionStatus {
    #[must_use]
    pub const fn allowed_next_states(self) -> &'static [Self] {
        match self {
            Self::Active => &[Self::WrappedUp, Self::Abandoned],
            Self::WrappedUp | Self::Abandoned => &[],
        }
    }

    #[must_use]
    pub fn can_transition_to(self, next: Self) -> bool {
        self.allowed_next_states().contains(&next)
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::WrappedUp => "wrapped_up",
            Self::Abandoned => "abandoned",
        }
    }
}

impl fmt::Display for SessionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

// ---------------------------------------------------------------------------
// StudyStatus
// ---------------------------------------------------------------------------

/// Status of a study.
///
/// ```text
/// active → concluding → completed
///                     → abandoned
/// active → abandoned
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum StudyStatus {
    Active,
    Concluding,
    Completed,
    Abandoned,
}

impl StudyStatus {
    #[must_use]
    pub const fn allowed_next_states(self) -> &'static [Self] {
        match self {
            Self::Active => &[Self::Concluding, Self::Abandoned],
            Self::Concluding => &[Self::Completed, Self::Abandoned],
            Self::Completed | Self::Abandoned => &[],
        }
    }

    #[must_use]
    pub fn can_transition_to(self, next: Self) -> bool {
        self.allowed_next_states().contains(&next)
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Concluding => "concluding",
            Self::Completed => "completed",
            Self::Abandoned => "abandoned",
        }
    }
}

impl fmt::Display for StudyStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

// ---------------------------------------------------------------------------
// StudyMethodology
// ---------------------------------------------------------------------------

/// Methodology used for a study.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum StudyMethodology {
    Explore,
    TestDriven,
    Compare,
}

impl StudyMethodology {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Explore => "explore",
            Self::TestDriven => "test_driven",
            Self::Compare => "compare",
        }
    }
}

impl fmt::Display for StudyMethodology {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

// ---------------------------------------------------------------------------
// CompatStatus
// ---------------------------------------------------------------------------

/// Status of a compatibility check between two packages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CompatStatus {
    Compatible,
    Incompatible,
    Conditional,
    Unknown,
}

impl CompatStatus {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Compatible => "compatible",
            Self::Incompatible => "incompatible",
            Self::Conditional => "conditional",
            Self::Unknown => "unknown",
        }
    }
}

impl fmt::Display for CompatStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

// ---------------------------------------------------------------------------
// AuditAction
// ---------------------------------------------------------------------------

/// Type of action recorded in the audit trail.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AuditAction {
    Created,
    Updated,
    StatusChanged,
    Linked,
    Unlinked,
    Tagged,
    Untagged,
    Indexed,
    SessionStart,
    SessionEnd,
    WrapUp,
}

impl AuditAction {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Updated => "updated",
            Self::StatusChanged => "status_changed",
            Self::Linked => "linked",
            Self::Unlinked => "unlinked",
            Self::Tagged => "tagged",
            Self::Untagged => "untagged",
            Self::Indexed => "indexed",
            Self::SessionStart => "session_start",
            Self::SessionEnd => "session_end",
            Self::WrapUp => "wrap_up",
        }
    }
}

impl fmt::Display for AuditAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

// ---------------------------------------------------------------------------
// EntityType
// ---------------------------------------------------------------------------

/// Type of entity in the system, used in audit trail and entity links.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EntityType {
    Session,
    Research,
    Finding,
    Hypothesis,
    Insight,
    Issue,
    Task,
    ImplLog,
    Compat,
    Study,
    Decision,
    EntityLink,
    Audit,
}

impl EntityType {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Session => "session",
            Self::Research => "research",
            Self::Finding => "finding",
            Self::Hypothesis => "hypothesis",
            Self::Insight => "insight",
            Self::Issue => "issue",
            Self::Task => "task",
            Self::ImplLog => "impl_log",
            Self::Compat => "compat",
            Self::Study => "study",
            Self::Decision => "decision",
            Self::EntityLink => "entity_link",
            Self::Audit => "audit",
        }
    }
}

impl fmt::Display for EntityType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

// ---------------------------------------------------------------------------
// Relation
// ---------------------------------------------------------------------------

/// Type of relationship between two entities in `entity_links`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Relation {
    Blocks,
    Validates,
    Debunks,
    Implements,
    RelatesTo,
    DerivedFrom,
    Triggers,
    Supersedes,
    DependsOn,
    FollowsPrecedent,
    OverridesPolicy,
}

impl Relation {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Blocks => "blocks",
            Self::Validates => "validates",
            Self::Debunks => "debunks",
            Self::Implements => "implements",
            Self::RelatesTo => "relates_to",
            Self::DerivedFrom => "derived_from",
            Self::Triggers => "triggers",
            Self::Supersedes => "supersedes",
            Self::DependsOn => "depends_on",
            Self::FollowsPrecedent => "follows_precedent",
            Self::OverridesPolicy => "overrides_policy",
        }
    }
}

impl fmt::Display for Relation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

// ---------------------------------------------------------------------------
// TrailOp
// ---------------------------------------------------------------------------

/// Operation type recorded in JSONL trail files.
///
/// The broad 8-variant set maps directly to audit action categories:
/// - Mutation ops: `Create`, `Update`, `Delete`
/// - Linking ops: `Link`, `Unlink`
/// - Tagging ops: `Tag`, `Untag`
/// - State machine op: `Transition`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TrailOp {
    Create,
    Update,
    Delete,
    Link,
    Unlink,
    Tag,
    Untag,
    Transition,
}

impl TrailOp {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Create => "create",
            Self::Update => "update",
            Self::Delete => "delete",
            Self::Link => "link",
            Self::Unlink => "unlink",
            Self::Tag => "tag",
            Self::Untag => "untag",
            Self::Transition => "transition",
        }
    }
}

impl fmt::Display for TrailOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- Serde roundtrip tests ---

    macro_rules! test_serde_roundtrip {
        ($name:ident, $ty:ty, $variant:expr, $expected_str:expr) => {
            #[test]
            fn $name() {
                let val = $variant;
                let json = serde_json::to_string(&val).unwrap();
                assert_eq!(json, format!("\"{}\"", $expected_str));
                let recovered: $ty = serde_json::from_str(&json).unwrap();
                assert_eq!(recovered, val);
            }
        };
    }

    test_serde_roundtrip!(confidence_high, Confidence, Confidence::High, "high");
    test_serde_roundtrip!(confidence_low, Confidence, Confidence::Low, "low");

    test_serde_roundtrip!(
        hyp_partially_confirmed,
        HypothesisStatus,
        HypothesisStatus::PartiallyConfirmed,
        "partially_confirmed"
    );
    test_serde_roundtrip!(
        hyp_unverified,
        HypothesisStatus,
        HypothesisStatus::Unverified,
        "unverified"
    );

    test_serde_roundtrip!(
        task_in_progress,
        TaskStatus,
        TaskStatus::InProgress,
        "in_progress"
    );
    test_serde_roundtrip!(task_done, TaskStatus, TaskStatus::Done, "done");

    test_serde_roundtrip!(issue_type_epic, IssueType, IssueType::Epic, "epic");
    test_serde_roundtrip!(
        issue_status_blocked,
        IssueStatus,
        IssueStatus::Blocked,
        "blocked"
    );

    test_serde_roundtrip!(
        research_resolved,
        ResearchStatus,
        ResearchStatus::Resolved,
        "resolved"
    );

    test_serde_roundtrip!(
        session_wrapped_up,
        SessionStatus,
        SessionStatus::WrappedUp,
        "wrapped_up"
    );

    test_serde_roundtrip!(
        study_concluding,
        StudyStatus,
        StudyStatus::Concluding,
        "concluding"
    );
    test_serde_roundtrip!(
        study_meth_td,
        StudyMethodology,
        StudyMethodology::TestDriven,
        "test_driven"
    );

    test_serde_roundtrip!(
        compat_conditional,
        CompatStatus,
        CompatStatus::Conditional,
        "conditional"
    );

    test_serde_roundtrip!(
        audit_status_changed,
        AuditAction,
        AuditAction::StatusChanged,
        "status_changed"
    );
    test_serde_roundtrip!(
        audit_session_start,
        AuditAction,
        AuditAction::SessionStart,
        "session_start"
    );

    test_serde_roundtrip!(
        entity_type_impl_log,
        EntityType,
        EntityType::ImplLog,
        "impl_log"
    );
    test_serde_roundtrip!(
        entity_type_entity_link,
        EntityType,
        EntityType::EntityLink,
        "entity_link"
    );

    test_serde_roundtrip!(
        relation_derived_from,
        Relation,
        Relation::DerivedFrom,
        "derived_from"
    );
    test_serde_roundtrip!(
        relation_relates_to,
        Relation,
        Relation::RelatesTo,
        "relates_to"
    );

    test_serde_roundtrip!(trail_op_create, TrailOp, TrailOp::Create, "create");
    test_serde_roundtrip!(
        trail_op_transition,
        TrailOp,
        TrailOp::Transition,
        "transition"
    );
    test_serde_roundtrip!(trail_op_untag, TrailOp, TrailOp::Untag, "untag");

    // --- Transition tests ---

    #[test]
    fn hypothesis_valid_transitions() {
        assert!(HypothesisStatus::Unverified.can_transition_to(HypothesisStatus::Analyzing));
        assert!(HypothesisStatus::Analyzing.can_transition_to(HypothesisStatus::Confirmed));
        assert!(HypothesisStatus::Analyzing.can_transition_to(HypothesisStatus::Debunked));
        assert!(HypothesisStatus::Analyzing.can_transition_to(HypothesisStatus::PartiallyConfirmed));
        assert!(HypothesisStatus::Analyzing.can_transition_to(HypothesisStatus::Inconclusive));
    }

    #[test]
    fn hypothesis_invalid_transitions() {
        assert!(!HypothesisStatus::Unverified.can_transition_to(HypothesisStatus::Confirmed));
        assert!(!HypothesisStatus::Confirmed.can_transition_to(HypothesisStatus::Analyzing));
        assert!(!HypothesisStatus::Debunked.can_transition_to(HypothesisStatus::Unverified));
    }

    #[test]
    fn task_valid_transitions() {
        assert!(TaskStatus::Open.can_transition_to(TaskStatus::InProgress));
        assert!(TaskStatus::InProgress.can_transition_to(TaskStatus::Done));
        assert!(TaskStatus::InProgress.can_transition_to(TaskStatus::Blocked));
        assert!(TaskStatus::Blocked.can_transition_to(TaskStatus::InProgress));
    }

    #[test]
    fn task_invalid_transitions() {
        assert!(!TaskStatus::Open.can_transition_to(TaskStatus::Done));
        assert!(!TaskStatus::Done.can_transition_to(TaskStatus::Open));
        assert!(!TaskStatus::Done.can_transition_to(TaskStatus::InProgress));
    }

    #[test]
    fn issue_valid_transitions() {
        assert!(IssueStatus::Open.can_transition_to(IssueStatus::InProgress));
        assert!(IssueStatus::InProgress.can_transition_to(IssueStatus::Done));
        assert!(IssueStatus::InProgress.can_transition_to(IssueStatus::Blocked));
        assert!(IssueStatus::InProgress.can_transition_to(IssueStatus::Abandoned));
        assert!(IssueStatus::Blocked.can_transition_to(IssueStatus::InProgress));
    }

    #[test]
    fn issue_invalid_transitions() {
        assert!(!IssueStatus::Open.can_transition_to(IssueStatus::Done));
        assert!(!IssueStatus::Abandoned.can_transition_to(IssueStatus::Open));
    }

    #[test]
    fn session_valid_transitions() {
        assert!(SessionStatus::Active.can_transition_to(SessionStatus::WrappedUp));
        assert!(SessionStatus::Active.can_transition_to(SessionStatus::Abandoned));
    }

    #[test]
    fn session_terminal_states() {
        assert!(SessionStatus::WrappedUp.allowed_next_states().is_empty());
        assert!(SessionStatus::Abandoned.allowed_next_states().is_empty());
    }

    #[test]
    fn study_valid_transitions() {
        assert!(StudyStatus::Active.can_transition_to(StudyStatus::Concluding));
        assert!(StudyStatus::Active.can_transition_to(StudyStatus::Abandoned));
        assert!(StudyStatus::Concluding.can_transition_to(StudyStatus::Completed));
        assert!(StudyStatus::Concluding.can_transition_to(StudyStatus::Abandoned));
    }

    #[test]
    fn study_invalid_transitions() {
        assert!(!StudyStatus::Active.can_transition_to(StudyStatus::Completed));
        assert!(!StudyStatus::Completed.can_transition_to(StudyStatus::Active));
    }

    #[test]
    fn research_valid_transitions() {
        assert!(ResearchStatus::Open.can_transition_to(ResearchStatus::InProgress));
        assert!(ResearchStatus::InProgress.can_transition_to(ResearchStatus::Resolved));
        assert!(ResearchStatus::InProgress.can_transition_to(ResearchStatus::Abandoned));
    }

    #[test]
    fn research_invalid_transitions() {
        assert!(!ResearchStatus::Open.can_transition_to(ResearchStatus::Resolved));
        assert!(!ResearchStatus::Resolved.can_transition_to(ResearchStatus::Open));
    }

    // --- Display / as_str tests ---

    #[test]
    fn display_matches_as_str() {
        assert_eq!(format!("{}", Confidence::High), "high");
        assert_eq!(
            format!("{}", HypothesisStatus::PartiallyConfirmed),
            "partially_confirmed"
        );
        assert_eq!(format!("{}", TaskStatus::InProgress), "in_progress");
        assert_eq!(format!("{}", IssueType::Spike), "spike");
        assert_eq!(format!("{}", IssueStatus::Abandoned), "abandoned");
        assert_eq!(format!("{}", ResearchStatus::Resolved), "resolved");
        assert_eq!(format!("{}", SessionStatus::WrappedUp), "wrapped_up");
        assert_eq!(format!("{}", StudyStatus::Concluding), "concluding");
        assert_eq!(format!("{}", StudyMethodology::TestDriven), "test_driven");
        assert_eq!(format!("{}", CompatStatus::Conditional), "conditional");
        assert_eq!(format!("{}", AuditAction::SessionStart), "session_start");
        assert_eq!(format!("{}", EntityType::ImplLog), "impl_log");
        assert_eq!(format!("{}", Relation::DerivedFrom), "derived_from");
        assert_eq!(format!("{}", TrailOp::Transition), "transition");
    }
}
