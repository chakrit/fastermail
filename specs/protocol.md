# MCP Protocol Layer

## Transport — stdio

- Read newline-delimited JSON-RPC 2.0 messages from stdin.
- Write newline-delimited JSON-RPC 2.0 messages to stdout.
- Never write non-JSON-RPC content to stdout. Logs go to stderr.

## JSON-RPC 2.0 Message Types

**Request** (client → server or server → client):
```json
{ "jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": { ... } }
```

**Response**:
```json
{ "jsonrpc": "2.0", "id": 1, "result": { ... } }
```

**Error response**:
```json
{ "jsonrpc": "2.0", "id": 1, "error": { "code": -32601, "message": "Method not found" } }
```

**Notification** (no `id`, no response expected):
```json
{ "jsonrpc": "2.0", "method": "notifications/initialized" }
```

## Initialization Handshake

Three-step sequence before any other messages:

1. Client sends `initialize` request with `protocolVersion`, `capabilities`, `clientInfo`.
2. Server responds with its `protocolVersion` (`2025-11-25`), `capabilities`, `serverInfo`.
3. Client sends `notifications/initialized` notification.

Server capabilities declared:
```json
{
  "tools": { "listChanged": false }
}
```

No resources, no prompts, no sampling — tools only.

## Methods the Server Must Handle

| Method                       | Type         | Description                    |
|------------------------------|--------------|--------------------------------|
| `initialize`                 | Request      | Handshake, return capabilities |
| `notifications/initialized`  | Notification | Client confirms init complete  |
| `ping`                       | Request      | Respond with `{ "result": {} }`|
| `tools/list`                 | Request      | Return all tool definitions    |
| `tools/call`                 | Request      | Execute a tool, return result  |

## Error Codes

| Code     | Meaning              |
|----------|----------------------|
| `-32700` | Parse error          |
| `-32600` | Invalid request      |
| `-32601` | Method not found     |
| `-32602` | Invalid params       |
| `-32603` | Internal error       |

Tool execution errors return a successful response with `isError: true` in the result content.
