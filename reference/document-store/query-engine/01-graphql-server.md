# GraphQL Server

## Purpose

The GraphQL server acts as the interface between external applications (primarily the Yioop search engine) and the document store. It receives HTTP requests containing GraphQL queries, executes them against the linear hash table, and returns the results.

## How It Works

### Request Flow

```text
Yioop Search Engine
    │
    │  HTTP POST (GraphQL query)
    ▼
┌──────────────────────────────┐
│       GraphQL Server         │
│                              │
│  1. Parse HTTP request       │
│  2. Extract GraphQL query    │
│  3. Validate against schema  │
│  4. Execute query            │
│  5. Fetch from hash table    │
│  6. Format response          │
│  7. Return HTTP response     │
│                              │
└──────────────────────────────┘
    │
    │  JSON response
    ▼
Yioop Search Engine
```

### Schema

The GraphQL schema defines the available queries for document retrieval. The primary operation is fetching a document by its key:

```text
Query {
  document(key: String!): Document
}

Document {
  key: String
  value: String
}
```

### Query Execution

When a query is received:

1. The GraphQL server parses the incoming HTTP request
2. The query string is extracted and validated against the schema
3. The resolver function is called with the provided key
4. The key is passed to the linear hash table for lookup
5. The hash table returns the document value (or null if not found)
6. The result is serialized as a JSON response and sent back

## Integration with Yioop

Yioop is a search engine that uses the document store as a backend for storing and retrieving web documents. The GraphQL API provides a flexible query interface that allows Yioop to:

- Retrieve individual documents by key
- Query document metadata
- Integrate with existing HTTP-based infrastructure

## Future Enhancements

- **Admin mutation queries** - GraphQL mutations for administrative operations (creating indexes, managing buckets)
- **Batch queries** - Fetching multiple documents in a single request
- **Subscription support** - Real-time updates when documents change

See [Planned Improvements](../future-work/02-improvements.md) for more details.

## Next Steps

- **[System Overview](../architecture/01-system-overview.md)** - How the GraphQL server fits in the architecture
- **[Performance Results](../experiments/01-performance-results.md)** - Query performance benchmarks
