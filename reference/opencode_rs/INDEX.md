# opencode_rs — Sub-Index

> Rust client for OpenCode LLM coding assistant (11 files)

### Root

|file|description|
|---|---|
|[README.md](README.md)|Getting started guide|
| |↳ [Key Features](README.md#key-features) · [Quick Start](README.md#quick-start) · [Documentation Structure](README.md#documentation-structure) · [Architecture](README.md#architecture) · [Requirements](README.md#requirements) · [Dependencies](README.md#dependencies) · [Version](README.md#version) · [License](README.md#license) · +3 more|

### [getting-started](getting-started/)

|file|description|
|---|---|
|[installation.md](getting-started/installation.md)|Installation — cargo add, feature flags|
| |↳ [Requirements](getting-started/installation.md#requirements) · [Adding to a Rust Project](getting-started/installation.md#adding-to-a-rust-project) · [Feature Flags](getting-started/installation.md#feature-flags) · [Development Dependencies](getting-started/installation.md#development-dependencies) · [Verifying Installation](getting-started/installation.md#verifying-installation) · [Installing OpenCode Server](getting-started/installation.md#installing-opencode-server) · [Docker Setup (Optional)](getting-started/installation.md#docker-setup-optional) · [IDE Setup](getting-started/installation.md#ide-setup) · +3 more|
|[quickstart.md](getting-started/quickstart.md)|Quickstart — first API call|
| |↳ [Prerequisites](getting-started/quickstart.md#prerequisites) · [Installation](getting-started/quickstart.md#installation) · [Your First Program](getting-started/quickstart.md#your-first-program) · [Understanding the Basics](getting-started/quickstart.md#understanding-the-basics) · [Next Steps](getting-started/quickstart.md#next-steps) · [Troubleshooting](getting-started/quickstart.md#troubleshooting) · [Resources](getting-started/quickstart.md#resources)|

### [core-concepts](core-concepts/)

|file|description|
|---|---|
|[architecture.md](core-concepts/architecture.md)|Architecture — client design, async patterns|
| |↳ [Design Principles](core-concepts/architecture.md#design-principles) · [System Architecture](core-concepts/architecture.md#system-architecture) · [Module Structure](core-concepts/architecture.md#module-structure) · [Data Flow](core-concepts/architecture.md#data-flow) · [Request Lifecycle](core-concepts/architecture.md#request-lifecycle) · [Concurrency Model](core-concepts/architecture.md#concurrency-model) · [Integration Points](core-concepts/architecture.md#integration-points) · [Performance Characteristics](core-concepts/architecture.md#performance-characteristics) · +3 more|
|[sessions.md](core-concepts/sessions.md)|Sessions — conversation management|
| |↳ [What is a Session?](core-concepts/sessions.md#what-is-a-session) · [Session Lifecycle](core-concepts/sessions.md#session-lifecycle) · [Creating Sessions](core-concepts/sessions.md#creating-sessions) · [Session Properties](core-concepts/sessions.md#session-properties) · [Managing Sessions](core-concepts/sessions.md#managing-sessions) · [Session Operations](core-concepts/sessions.md#session-operations) · [Session Context](core-concepts/sessions.md#session-context) · [Session Messages](core-concepts/sessions.md#session-messages) · +6 more|

### [api-reference](api-reference/)

|file|description|
|---|---|
|[client.md](api-reference/client.md)|Client — OpenCodeClient struct and methods|
| |↳ [Client Overview](api-reference/client.md#client-overview) · [ClientBuilder](api-reference/client.md#clientbuilder) · [Client Methods](api-reference/client.md#client-methods) · [Thread Safety](api-reference/client.md#thread-safety) · [Error Handling](api-reference/client.md#error-handling) · [Complete Example](api-reference/client.md#complete-example) · [Advanced Usage](api-reference/client.md#advanced-usage)|
|[http-apis.md](api-reference/http-apis.md)|HTTP APIs — REST endpoint reference|
| |↳ [HTTP Client](api-reference/http-apis.md#http-client) · [API Modules](api-reference/http-apis.md#api-modules) · [All API Modules](api-reference/http-apis.md#all-api-modules) · [Error Handling](api-reference/http-apis.md#error-handling)|
|[sse.md](api-reference/sse.md)|SSE — Server-Sent Events streaming|
| |↳ [SseSubscriber](api-reference/sse.md#ssesubscriber) · [Subscription Types](api-reference/sse.md#subscription-types) · [SseSubscription](api-reference/sse.md#ssesubscription) · [SseOptions](api-reference/sse.md#sseoptions) · [Event Types](api-reference/sse.md#event-types) · [Handling Events](api-reference/sse.md#handling-events) · [Reconnection](api-reference/sse.md#reconnection) · [Complete Example](api-reference/sse.md#complete-example) · +3 more|

### [configuration](configuration/)

|file|description|
|---|---|
|[client-config.md](configuration/client-config.md)|Configuration — client options, auth|
| |↳ [Basic Configuration](configuration/client-config.md#basic-configuration) · [Configuration Options](configuration/client-config.md#configuration-options) · [Environment Variables](configuration/client-config.md#environment-variables) · [Feature Flags](configuration/client-config.md#feature-flags) · [Multiple Clients](configuration/client-config.md#multiple-clients)|

### [examples](examples/)

|file|description|
|---|---|
|[basic-usage.md](examples/basic-usage.md)|Examples — complete usage patterns|
| |↳ [Example 1: Simple Session](examples/basic-usage.md#example-1-simple-session) · [Example 2: Session with Events](examples/basic-usage.md#example-2-session-with-events) · [Example 3: List and Manage Sessions](examples/basic-usage.md#example-3-list-and-manage-sessions)|

### [types](types/)

|file|description|
|---|---|
|[core-types.md](types/core-types.md)|Types — request/response type definitions|
| |↳ [Session Types](types/core-types.md#session-types) · [Message Types](types/core-types.md#message-types) · [Event Types](types/core-types.md#event-types) · [Tool Types](types/core-types.md#tool-types) · [Provider Types](types/core-types.md#provider-types) · [Error Types](types/core-types.md#error-types) · [Common Patterns](types/core-types.md#common-patterns)|

---
*11 files*
