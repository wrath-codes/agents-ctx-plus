package registry

import (
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"time"
)

// Package represents a package from any registry.
type Package struct {
	Name         string   `json:"name"`
	Version      string   `json:"version"`
	Description  string   `json:"description"`
	Homepage     string   `json:"homepage,omitempty"`
	Repository   string   `json:"repository,omitempty"`
	License      string   `json:"license,omitempty"`
	Downloads    int64    `json:"downloads,omitempty"`
	Registry     string   `json:"registry"` // "crates.io", "npm", "pypi", "hex"
	Keywords     []string `json:"keywords,omitempty"`
	LastUpdated  string   `json:"last_updated,omitempty"`
	ReadmeURL    string   `json:"readme_url,omitempty"`
	DocumentsURL string   `json:"documents_url,omitempty"`
}

// SearchResult holds search results from a registry.
type SearchResult struct {
	Registry string    `json:"registry"`
	Query    string    `json:"query"`
	Total    int       `json:"total"`
	Packages []Package `json:"packages"`
}

// Client queries package registries.
type Client struct {
	http *http.Client
}

// NewClient creates a new registry client.
func NewClient() *Client {
	return &Client{
		http: &http.Client{
			Timeout: 15 * time.Second,
		},
	}
}

// SearchCratesIO searches crates.io for Rust crates.
func (c *Client) SearchCratesIO(ctx context.Context, query string, limit int) (*SearchResult, error) {
	if limit <= 0 {
		limit = 10
	}
	u := fmt.Sprintf("https://crates.io/api/v1/crates?q=%s&per_page=%d&sort=downloads",
		url.QueryEscape(query), limit)

	body, err := c.get(ctx, u, map[string]string{
		"User-Agent": "beads-workflow-system (research-agent)",
	})
	if err != nil {
		return nil, fmt.Errorf("crates.io search failed: %w", err)
	}

	var resp struct {
		Crates []struct {
			Name          string   `json:"name"`
			Description   string   `json:"description"`
			MaxVersion    string   `json:"max_version"`
			Homepage      string   `json:"homepage"`
			Repository    string   `json:"repository"`
			Downloads     int64    `json:"downloads"`
			RecentDL      int64    `json:"recent_downloads"`
			Keywords      []string `json:"keywords"`
			UpdatedAt     string   `json:"updated_at"`
			Documentation string   `json:"documentation"`
		} `json:"crates"`
		Meta struct {
			Total int `json:"total"`
		} `json:"meta"`
	}
	if err := json.Unmarshal(body, &resp); err != nil {
		return nil, fmt.Errorf("crates.io parse failed: %w", err)
	}

	result := &SearchResult{
		Registry: "crates.io",
		Query:    query,
		Total:    resp.Meta.Total,
	}
	for _, cr := range resp.Crates {
		readmeURL := fmt.Sprintf("https://crates.io/api/v1/crates/%s/%s/readme", cr.Name, cr.MaxVersion)
		docsURL := cr.Documentation
		if docsURL == "" {
			docsURL = fmt.Sprintf("https://docs.rs/%s/%s", cr.Name, cr.MaxVersion)
		}
		result.Packages = append(result.Packages, Package{
			Name:         cr.Name,
			Version:      cr.MaxVersion,
			Description:  cr.Description,
			Homepage:     cr.Homepage,
			Repository:   cr.Repository,
			License:      "",
			Downloads:    cr.Downloads,
			Registry:     "crates.io",
			Keywords:     cr.Keywords,
			LastUpdated:  cr.UpdatedAt,
			ReadmeURL:    readmeURL,
			DocumentsURL: docsURL,
		})
	}
	return result, nil
}

// SearchNPM searches the npm registry for JavaScript/TypeScript packages.
func (c *Client) SearchNPM(ctx context.Context, query string, limit int) (*SearchResult, error) {
	if limit <= 0 {
		limit = 10
	}
	u := fmt.Sprintf("https://registry.npmjs.org/-/v1/search?text=%s&size=%d",
		url.QueryEscape(query), limit)

	body, err := c.get(ctx, u, nil)
	if err != nil {
		return nil, fmt.Errorf("npm search failed: %w", err)
	}

	var resp struct {
		Total   int `json:"total"`
		Objects []struct {
			Package struct {
				Name        string   `json:"name"`
				Version     string   `json:"version"`
				Description string   `json:"description"`
				Keywords    []string `json:"keywords"`
				Links       struct {
					Homepage   string `json:"homepage"`
					Repository string `json:"repository"`
					Npm        string `json:"npm"`
				} `json:"links"`
				Date string `json:"date"`
			} `json:"package"`
		} `json:"objects"`
	}
	if err := json.Unmarshal(body, &resp); err != nil {
		return nil, fmt.Errorf("npm parse failed: %w", err)
	}

	result := &SearchResult{
		Registry: "npm",
		Query:    query,
		Total:    resp.Total,
	}
	for _, obj := range resp.Objects {
		p := obj.Package
		pkg := Package{
			Name:         p.Name,
			Version:      p.Version,
			Description:  p.Description,
			Homepage:     p.Links.Homepage,
			Repository:   p.Links.Repository,
			Registry:     "npm",
			Keywords:     p.Keywords,
			LastUpdated:  p.Date,
			DocumentsURL: p.Links.Npm,
		}
		// Fetch download count (best-effort, don't fail the search).
		if dl, err := c.getNPMDownloads(ctx, p.Name); err == nil {
			pkg.Downloads = dl
		}
		result.Packages = append(result.Packages, pkg)
	}
	return result, nil
}

// getNPMDownloads fetches last-month download count for an npm package.
func (c *Client) getNPMDownloads(ctx context.Context, name string) (int64, error) {
	u := fmt.Sprintf("https://api.npmjs.org/downloads/point/last-month/%s", url.PathEscape(name))
	body, err := c.get(ctx, u, nil)
	if err != nil {
		return 0, err
	}
	var resp struct {
		Downloads int64 `json:"downloads"`
	}
	if err := json.Unmarshal(body, &resp); err != nil {
		return 0, err
	}
	return resp.Downloads, nil
}

// SearchPyPI searches PyPI for Python packages.
func (c *Client) SearchPyPI(ctx context.Context, query string, limit int) (*SearchResult, error) {
	// PyPI doesn't have a real search API, but we can query individual packages.
	// We'll use the warehouse simple API for search-like behavior via the JSON API.
	u := fmt.Sprintf("https://pypi.org/pypi/%s/json", url.PathEscape(query))

	body, err := c.get(ctx, u, nil)
	if err != nil {
		// PyPI returns 404 for unknown packages; treat as empty result.
		return &SearchResult{
			Registry: "pypi",
			Query:    query,
			Total:    0,
		}, nil
	}

	var resp struct {
		Info struct {
			Name           string `json:"name"`
			Version        string `json:"version"`
			Summary        string `json:"summary"`
			HomePage       string `json:"home_page"`
			ProjectURL     string `json:"project_url"`
			License        string `json:"license"`
			Keywords       string `json:"keywords"`
			PackageURL     string `json:"package_url"`
			DocsURL        string `json:"docs_url"`
			RequiresPython string `json:"requires_python"`
		} `json:"info"`
	}
	if err := json.Unmarshal(body, &resp); err != nil {
		return nil, fmt.Errorf("pypi parse failed: %w", err)
	}

	result := &SearchResult{
		Registry: "pypi",
		Query:    query,
		Total:    1,
	}
	info := resp.Info
	var keywords []string
	if info.Keywords != "" {
		keywords = []string{info.Keywords}
	}
	result.Packages = append(result.Packages, Package{
		Name:         info.Name,
		Version:      info.Version,
		Description:  info.Summary,
		Homepage:     info.HomePage,
		License:      info.License,
		Registry:     "pypi",
		Keywords:     keywords,
		DocumentsURL: info.DocsURL,
	})
	return result, nil
}

// SearchHex searches Hex.pm for Elixir/Erlang packages.
func (c *Client) SearchHex(ctx context.Context, query string, limit int) (*SearchResult, error) {
	if limit <= 0 {
		limit = 10
	}
	u := fmt.Sprintf("https://hex.pm/api/packages?search=%s&sort=downloads&page=1&per_page=%d",
		url.QueryEscape(query), limit)

	body, err := c.get(ctx, u, nil)
	if err != nil {
		return nil, fmt.Errorf("hex search failed: %w", err)
	}

	var packages []struct {
		Name string `json:"name"`
		Meta struct {
			Description string            `json:"description"`
			Licenses    []string          `json:"licenses"`
			Links       map[string]string `json:"links"`
		} `json:"meta"`
		Downloads struct {
			All int64 `json:"all"`
		} `json:"downloads"`
		Releases []struct {
			Version string `json:"version"`
		} `json:"releases"`
		UpdatedAt string `json:"updated_at"`
	}
	if err := json.Unmarshal(body, &packages); err != nil {
		return nil, fmt.Errorf("hex parse failed: %w", err)
	}

	result := &SearchResult{
		Registry: "hex",
		Query:    query,
		Total:    len(packages),
	}
	for _, p := range packages {
		version := ""
		if len(p.Releases) > 0 {
			version = p.Releases[0].Version
		}
		license := ""
		if len(p.Meta.Licenses) > 0 {
			license = p.Meta.Licenses[0]
		}
		homepage := p.Meta.Links["GitHub"]
		if homepage == "" {
			homepage = p.Meta.Links["Homepage"]
		}
		result.Packages = append(result.Packages, Package{
			Name:         p.Name,
			Version:      version,
			Description:  p.Meta.Description,
			Homepage:     homepage,
			License:      license,
			Downloads:    p.Downloads.All,
			Registry:     "hex",
			LastUpdated:  p.UpdatedAt,
			DocumentsURL: fmt.Sprintf("https://hexdocs.pm/%s", p.Name),
		})
	}
	return result, nil
}

// SearchAll searches all registries concurrently and returns combined results.
func (c *Client) SearchAll(ctx context.Context, query string, limit int) ([]SearchResult, error) {
	type result struct {
		sr  *SearchResult
		err error
	}

	registries := []struct {
		name   string
		search func(context.Context, string, int) (*SearchResult, error)
	}{
		{"crates.io", c.SearchCratesIO},
		{"npm", c.SearchNPM},
		{"hex", c.SearchHex},
	}

	ch := make(chan result, len(registries))
	for _, r := range registries {
		go func(name string, fn func(context.Context, string, int) (*SearchResult, error)) {
			sr, err := fn(ctx, query, limit)
			ch <- result{sr, err}
		}(r.name, r.search)
	}

	var results []SearchResult
	for range registries {
		res := <-ch
		if res.err != nil {
			// Best-effort: skip failed registries.
			continue
		}
		if res.sr != nil && len(res.sr.Packages) > 0 {
			results = append(results, *res.sr)
		}
	}

	return results, nil
}

// FetchReadme fetches a README from a URL and returns the content as a string.
func (c *Client) FetchReadme(ctx context.Context, readmeURL string) (string, error) {
	body, err := c.get(ctx, readmeURL, map[string]string{
		"User-Agent": "beads-workflow-system (research-agent)",
	})
	if err != nil {
		return "", fmt.Errorf("failed to fetch readme: %w", err)
	}
	// Truncate to ~8KB to avoid overwhelming LLM context.
	content := string(body)
	if len(content) > 8192 {
		content = content[:8192] + "\n...(truncated)"
	}
	return content, nil
}

// get performs an HTTP GET and returns the response body.
func (c *Client) get(ctx context.Context, url string, headers map[string]string) ([]byte, error) {
	req, err := http.NewRequestWithContext(ctx, http.MethodGet, url, nil)
	if err != nil {
		return nil, err
	}
	req.Header.Set("Accept", "application/json")
	for k, v := range headers {
		req.Header.Set(k, v)
	}

	resp, err := c.http.Do(req)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return nil, fmt.Errorf("HTTP %d from %s", resp.StatusCode, url)
	}

	return io.ReadAll(resp.Body)
}
