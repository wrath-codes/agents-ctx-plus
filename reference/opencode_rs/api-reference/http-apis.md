# HTTP APIs

The HTTP module provides low-level access to all OpenCode REST API endpoints. While the `Client` offers a high-level interface, you can use individual API modules for more control.

## HTTP Client

The `HttpClient` handles all HTTP communication:

```rust
use opencode_rs::http::{HttpClient, HttpConfig};

let config = HttpConfig {
    base_url: "http://127.0.0.1:4096".to_string(),
    directory: Some("/path/to/project".to_string()),
    timeout_secs: 300,
};

let http_client = HttpClient::new(config)?;
```

## API Modules

### Sessions API

Manage coding sessions.

```rust
use opencode_rs::http::sessions::SessionsApi;
use opencode_rs::types::session::{
    CreateSessionRequest, UpdateSessionRequest, 
    RevertRequest, SummarizeRequest
};

let sessions = client.sessions();
```

#### Create Session

```rust
let request = CreateSessionRequest {
    description: Some("Implement feature".to_string()),
    initial_prompt: Some("Write code".to_string()),
    provider: Some(Provider::Claude),
    model: Some("claude-3-opus".to_string()),
    agent: Some("default".to_string()),
    tools: Some(vec!["file".to_string(), "shell".to_string()]),
    ephemeral: Some(false),
};

let session = sessions.create(request).await?;
```

**Endpoint:** `POST /sessions`

**Returns:** `Session`

#### List Sessions

```rust
let sessions = sessions.list().await?;

for session in sessions {
    println!("{}: {:?}", session.id, session.status);
}
```

**Endpoint:** `GET /sessions`

**Returns:** `Vec<Session>`

### Messages API

Send and manage messages in sessions.

```rust
use opencode_rs::http::messages::MessagesApi;
use opencode_rs::types::message::{
    PromptRequest, CommandRequest, ShellRequest,
    PromptPart, Part
};

let messages = client.messages();
```

#### Send Prompt

```rust
let request = PromptRequest {
    session_id: session_id.clone(),
    content: vec![
        PromptPart::Text {
            text: "Review this code".to_string(),
        },
    ],
    ephemeral: Some(false),
};

let message = messages.create_prompt(request).await?;
```

**Endpoint:** `POST /messages/prompt`

**Returns:** `Message`

#### List Messages

```rust
let messages_list = messages.list(&session_id).await?;
```

**Endpoint:** `GET /sessions/{session_id}/messages`

**Returns:** `Vec<Message>`

### Files API

File operations within the working directory.

```rust
use opencode_rs::http::files::FilesApi;

let files = client.files();
```

#### Read File

```rust
let content = files.read("src/main.rs").await?;
```

**Endpoint:** `GET /files/{path}`

**Returns:** `String`

#### Write File

```rust
files.write("src/main.rs", "fn main() {}").await?;
```

**Endpoint:** `PUT /files/{path}`

### Tools API

Manage available tools and agents.

```rust
use opencode_rs::http::tools::ToolsApi;

let tools = client.tools();
```

#### List Tools

```rust
let tools_list = tools.list().await?;

for tool in tools_list {
    println!("{}: {}", tool.id, tool.description);
}
```

**Endpoint:** `GET /tools`

**Returns:** `Vec<Tool>`

## All API Modules

| Module | Description | Key Methods |
|--------|-------------|-------------|
| `sessions` | Session management | `create`, `list`, `get`, `update`, `delete` |
| `messages` | Message operations | `create_prompt`, `create_command`, `list` |
| `parts` | Content parts | `list` |
| `files` | File operations | `read`, `write`, `delete`, `list` |
| `tools` | Tool management | `list`, `get`, `list_agents` |
| `mcp` | MCP operations | `list_servers`, `call_tool` |
| `providers` | Provider info | `list`, `get` |
| `permissions` | Permission handling | `list_pending`, `grant`, `deny` |
| `config` | Configuration | `get`, `update` |
| `project` | Project info | `info` |
| `worktree` | Worktree status | `status` |
| `find` | Search operations | `files`, `symbols` |
| `pty` | Terminal operations | `execute` |
| `misc` | Miscellaneous | `health` |

## Error Handling

All API methods return `Result<T, OpencodeError>`:

```rust
match client.sessions().create(request).await {
    Ok(session) => println!("Created: {}", session.id),
    Err(OpencodeError::Api { code, message }) => {
        eprintln!("API Error {}: {}", code, message);
    }
    Err(e) => eprintln!("Error: {}", e),
}
```