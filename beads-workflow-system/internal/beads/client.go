package beads

import (
	"context"
	"fmt"
	"time"

	bd "github.com/steveyegge/beads"
	"github.com/your-org/beads-workflow-system/pkg/models"
)

// Client provides an interface to the Beads issue tracking system.
// It wraps the real beads Storage API.
type Client struct {
	storage bd.Storage
}

// NewClient creates a new Beads client backed by the real beads SQLite storage.
// dbPath should point to the .beads/beads.db file.
func NewClient(dbPath string) (*Client, error) {
	ctx := context.Background()
	storage, err := bd.NewSQLiteStorage(ctx, dbPath)
	if err != nil {
		return nil, fmt.Errorf("failed to open beads storage at %s: %w", dbPath, err)
	}
	return &Client{storage: storage}, nil
}

// NewClientFromStorage creates a Client from an existing beads Storage instance.
func NewClientFromStorage(storage bd.Storage) *Client {
	return &Client{storage: storage}
}

// Storage returns the underlying beads Storage for direct access.
func (c *Client) Storage() bd.Storage {
	return c.storage
}

// CreateIssue creates a new beads issue.
func (c *Client) CreateIssue(ctx context.Context, req *models.CreateIssueRequest) (*models.Issue, error) {
	issue := &bd.Issue{
		Title:       req.Title,
		Description: req.Description,
		IssueType:   bd.IssueType(req.Type),
		Priority:    req.Priority,
		Assignee:    req.Assignee,
		Labels:      req.Labels,
		Status:      bd.StatusOpen,
	}

	actor := req.Assignee
	if actor == "" {
		actor = "workflow-system"
	}

	err := c.storage.CreateIssue(ctx, issue, actor)
	if err != nil {
		return nil, fmt.Errorf("failed to create beads issue: %w", err)
	}

	return issueToModel(issue), nil
}

// GetIssue retrieves an existing beads issue by ID.
func (c *Client) GetIssue(ctx context.Context, issueID string) (*models.Issue, error) {
	issue, err := c.storage.GetIssue(ctx, issueID)
	if err != nil {
		return nil, fmt.Errorf("failed to get beads issue %s: %w", issueID, err)
	}
	return issueToModel(issue), nil
}

// UpdateIssueStatus updates the status of an existing beads issue.
func (c *Client) UpdateIssueStatus(ctx context.Context, issueID string, status string) error {
	updates := map[string]interface{}{
		"status": status,
	}
	return c.storage.UpdateIssue(ctx, issueID, updates, "workflow-system")
}

// UpdateIssue applies partial updates to an existing beads issue.
func (c *Client) UpdateIssue(ctx context.Context, issueID string, updates map[string]interface{}) error {
	return c.storage.UpdateIssue(ctx, issueID, updates, "workflow-system")
}

// ClaimIssue atomically claims an issue (sets assignee + in_progress).
func (c *Client) ClaimIssue(ctx context.Context, issueID string, agentID string) error {
	return c.storage.ClaimIssue(ctx, issueID, agentID)
}

// CloseIssue closes a beads issue with a reason.
func (c *Client) CloseIssue(ctx context.Context, issueID string, reason string) error {
	return c.storage.CloseIssue(ctx, issueID, reason, "workflow-system", "")
}

// DeleteIssue deletes a beads issue.
func (c *Client) DeleteIssue(ctx context.Context, issueID string) error {
	return c.storage.DeleteIssue(ctx, issueID)
}

// SearchIssues searches beads issues with a query and filter.
func (c *Client) SearchIssues(ctx context.Context, query string, filter *models.IssueFilter) ([]*models.Issue, error) {
	bdFilter := modelFilterToBeads(filter)
	issues, err := c.storage.SearchIssues(ctx, query, bdFilter)
	if err != nil {
		return nil, fmt.Errorf("failed to search beads issues: %w", err)
	}
	return issuesToModels(issues), nil
}

// GetReadyWork returns issues with no open blockers that are ready for work.
func (c *Client) GetReadyWork(ctx context.Context) ([]*models.Issue, error) {
	workFilter := bd.WorkFilter{}
	issues, err := c.storage.GetReadyWork(ctx, workFilter)
	if err != nil {
		return nil, fmt.Errorf("failed to get ready work: %w", err)
	}
	return issuesToModels(issues), nil
}

// IsBlocked checks whether an issue is blocked. Returns blockerIDs if blocked.
func (c *Client) IsBlocked(ctx context.Context, issueID string) (bool, []string, error) {
	return c.storage.IsBlocked(ctx, issueID)
}

// AddDependency adds a dependency between two issues.
func (c *Client) AddDependency(ctx context.Context, issueID string, dependsOnID string, depType string) error {
	dep := &bd.Dependency{
		IssueID:     issueID,
		DependsOnID: dependsOnID,
		Type:        bd.DependencyType(depType),
		CreatedAt:   time.Now(),
	}
	return c.storage.AddDependency(ctx, dep, "workflow-system")
}

// RemoveDependency removes a dependency between two issues.
func (c *Client) RemoveDependency(ctx context.Context, issueID string, dependsOnID string) error {
	return c.storage.RemoveDependency(ctx, issueID, dependsOnID, "workflow-system")
}

// GetDependencies returns issues that the given issue depends on.
func (c *Client) GetDependencies(ctx context.Context, issueID string) ([]*models.Issue, error) {
	issues, err := c.storage.GetDependencies(ctx, issueID)
	if err != nil {
		return nil, fmt.Errorf("failed to get dependencies for %s: %w", issueID, err)
	}
	return issuesToModels(issues), nil
}

// GetDependents returns issues that depend on the given issue.
func (c *Client) GetDependents(ctx context.Context, issueID string) ([]*models.Issue, error) {
	issues, err := c.storage.GetDependents(ctx, issueID)
	if err != nil {
		return nil, fmt.Errorf("failed to get dependents for %s: %w", issueID, err)
	}
	return issuesToModels(issues), nil
}

// GetDependencyTree returns the dependency tree for an issue.
func (c *Client) GetDependencyTree(ctx context.Context, issueID string, maxDepth int) ([]*bd.TreeNode, error) {
	return c.storage.GetDependencyTree(ctx, issueID, maxDepth, false, false)
}

// AddComment adds a comment to a beads issue.
func (c *Client) AddComment(ctx context.Context, issueID string, comment string) error {
	return c.storage.AddComment(ctx, issueID, "workflow-system", comment)
}

// GetEvents returns events for an issue.
func (c *Client) GetEvents(ctx context.Context, issueID string, limit int) ([]*bd.Event, error) {
	return c.storage.GetEvents(ctx, issueID, limit)
}

// AddLabel adds a label to an issue.
func (c *Client) AddLabel(ctx context.Context, issueID string, label string) error {
	return c.storage.AddLabel(ctx, issueID, label, "workflow-system")
}

// RemoveLabel removes a label from an issue.
func (c *Client) RemoveLabel(ctx context.Context, issueID string, label string) error {
	return c.storage.RemoveLabel(ctx, issueID, label, "workflow-system")
}

// GetLabels returns labels for an issue.
func (c *Client) GetLabels(ctx context.Context, issueID string) ([]string, error) {
	return c.storage.GetLabels(ctx, issueID)
}

// Close closes the beads client and releases resources.
func (c *Client) Close() error {
	return c.storage.Close()
}

// --- Conversion helpers ---

func issueToModel(issue *bd.Issue) *models.Issue {
	if issue == nil {
		return nil
	}
	return &models.Issue{
		ID:          issue.ID,
		Title:       issue.Title,
		Description: issue.Description,
		Type:        string(issue.IssueType),
		Priority:    issue.Priority,
		Assignee:    issue.Assignee,
		Labels:      issue.Labels,
		Status:      string(issue.Status),
		CreatedAt:   issue.CreatedAt,
		UpdatedAt:   issue.UpdatedAt,
	}
}

func issuesToModels(issues []*bd.Issue) []*models.Issue {
	result := make([]*models.Issue, len(issues))
	for i, issue := range issues {
		result[i] = issueToModel(issue)
	}
	return result
}

func modelFilterToBeads(filter *models.IssueFilter) bd.IssueFilter {
	if filter == nil {
		return bd.IssueFilter{}
	}
	f := bd.IssueFilter{
		Limit: filter.Limit,
	}
	if filter.Status != "" {
		status := bd.Status(filter.Status)
		f.Status = &status
	}
	if filter.Type != "" {
		issueType := bd.IssueType(filter.Type)
		f.IssueType = &issueType
	}
	if filter.Priority != 0 {
		f.Priority = &filter.Priority
	}
	if filter.Assignee != "" {
		f.Assignee = &filter.Assignee
	}
	if len(filter.Labels) > 0 {
		f.Labels = filter.Labels
	}
	if filter.Search != "" {
		f.TitleSearch = filter.Search
	}
	return f
}
