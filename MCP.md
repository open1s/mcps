# Model Context Protocol (MCP): Building Blocks

The Model Context Protocol (MCP) is a standardized way for AI assistants to communicate with tools, data sources, and other services. This document explains MCP's core concepts using visual diagrams to help you understand how it works, step by step, like building with LEGO blocks.

## 1. The Big Picture: Client-Server Architecture

At its core, MCP follows a client-server architecture that connects AI applications with tools and services.

```mermaid
graph TD
    subgraph "Host Application"
        Client[MCP Client]
    end
    
    subgraph "Tool Provider"
        Server[MCP Server]
        Tool1[Tool 1]
        Tool2[Tool 2]
        Tool3[Tool 3]
        Server --> Tool1
        Server --> Tool2
        Server --> Tool3
    end
    
    Client <--> Server
```

### Key Components:
- **Host**: An application like Claude Desktop or an IDE that initiates connections
- **Client**: Maintains a 1:1 connection with a server, embedded in the host application
- **Server**: Provides tools, context, and capabilities to clients
- **Tools**: Specific functionalities provided by the server (e.g., web search, code execution)

## 2. Building the Connection: Transport Layer

The transport layer is like the foundation of our LEGO tower - it handles the actual communication between clients and servers.

```mermaid
graph TD
    subgraph "Client Side"
        ClientApp[Client Application]
        ClientTransport[Transport Layer]
        ClientProtocol[Protocol Layer]
        
        ClientApp --> ClientProtocol
        ClientProtocol --> ClientTransport
    end
    
    subgraph "Server Side"
        ServerTransport[Transport Layer]
        ServerProtocol[Protocol Layer]
        ServerApp[Server Application]
        
        ServerTransport --> ServerProtocol
        ServerProtocol --> ServerApp
    end
    
    ClientTransport <--> |JSON-RPC 2.0| ServerTransport
```

### Transport Types:

#### Stdio Transport (Local Communication)
```mermaid
sequenceDiagram
    participant Client
    participant Server
    
    Client->>Server: Write to stdin
    Server->>Client: Read from stdout
    Server->>Client: Write to stdout
    Client->>Server: Read from stdin
```

#### SSE Transport (Web Communication)
```mermaid
sequenceDiagram
    participant Client
    participant Server
    
    Client->>Server: HTTP POST /messages
    Server->>Client: SSE Event Stream
    Client->>Server: HTTP POST /messages
    Server->>Client: SSE Event Stream
```

## 3. Speaking the Same Language: Message Format

MCP uses JSON-RPC 2.0 as its message format. Think of these messages as specialized LEGO pieces that fit together in specific ways.

```mermaid
graph TD
    subgraph "JSON-RPC 2.0 Messages"
        Request[Request]
        Response[Response]
        Notification[Notification]
        Error[Error]
    end
```

### Message Types:

#### Request (Client → Server or Server → Client)
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tool_call",
  "params": {
    "name": "web_search",
    "parameters": {
      "query": "weather in San Francisco"
    }
  }
}
```

#### Response (Success)
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "temperature": "72°F",
    "conditions": "Partly cloudy"
  }
}
```

#### Response (Error)
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32603,
    "message": "Internal error",
    "data": {
      "details": "Search service unavailable"
    }
  }
}
```

#### Notification (One-way message)
```json
{
  "jsonrpc": "2.0",
  "method": "progress",
  "params": {
    "percent": 50,
    "message": "Processing query..."
  }
}
```

## 4. The Connection Lifecycle: Building Our Tower

The MCP connection follows a specific lifecycle, like building a LEGO tower step by step.

```mermaid
stateDiagram-v2
    [*] --> Initializing
    Initializing --> Connected: initialize request/response
    Connected --> Active: initialized notification
    Active --> Terminating: close request
    Terminating --> [*]: close response
```

### Step 1: Initialization
```mermaid
sequenceDiagram
    participant Client
    participant Server
    
    Client->>Server: initialize request
    Note right of Client: Protocol version & capabilities
    Server->>Client: initialize response
    Note left of Server: Server capabilities & info
    Client->>Server: initialized notification
    Note right of Client: Acknowledgment
```

### Step 2: Message Exchange
```mermaid
sequenceDiagram
    participant Client
    participant Server
    
    Client->>Server: tool_call request
    Server->>Client: progress notification
    Note left of Server: Optional progress updates
    Server->>Client: tool_call response
    
    Server->>Client: request_input request
    Client->>Server: request_input response
```

### Step 3: Termination
```mermaid
sequenceDiagram
    participant Client
    participant Server
    
    Client->>Server: shutdown request
    Server->>Client: shutdown response
    Note left of Server: Clean shutdown
```

## 5. Tools: The Building Blocks of Functionality

Tools are the functional building blocks that servers provide to clients.

```mermaid
graph TD
    subgraph "MCP Server"
        Server[Server]
        ToolRegistry[Tool Registry]
        
        Tool1[Web Search Tool]
        Tool2[Code Execution Tool]
        Tool3[Database Query Tool]
        
        Server --> ToolRegistry
        ToolRegistry --> Tool1
        ToolRegistry --> Tool2
        ToolRegistry --> Tool3
    end
    
    Client[Client] --> Server
```

### Tool Definition Example
```json
{
  "name": "web_search",
  "description": "Search the web for information",
  "input_schema": {
    "type": "object",
    "properties": {
      "query": {
        "type": "string",
        "description": "The search query"
      },
      "num_results": {
        "type": "integer",
        "description": "Number of results to return",
        "default": 5
      }
    },
    "required": ["query"]
  }
}
```

## 6. Error Handling: When Blocks Don't Fit

Error handling in MCP is like knowing what to do when LEGO pieces don't fit together properly.

```mermaid
graph TD
    Error[Error Occurs] --> Detect[Detect Error]
    Detect --> Classify[Classify Error]
    
    Classify --> Protocol[Protocol Error]
    Classify --> Transport[Transport Error]
    Classify --> Application[Application Error]
    
    Protocol --> ErrorResponse[Send Error Response]
    Transport --> CloseConnection[Close Connection]
    Application --> CustomHandler[Custom Error Handler]
    
    ErrorResponse --> Recover[Recover or Retry]
    CloseConnection --> Reconnect[Reconnect if Possible]
    CustomHandler --> ApplicationSpecific[Application-Specific Recovery]
```

### Standard Error Codes
```mermaid
graph TD
    subgraph "Standard JSON-RPC Error Codes"
        ParseError["-32700: Parse Error"]
        InvalidRequest["-32600: Invalid Request"]
        MethodNotFound["-32601: Method Not Found"]
        InvalidParams["-32602: Invalid Params"]
        InternalError["-32603: Internal Error"]
    end
    
    subgraph "Custom Error Codes"
        CustomErrors["Application-specific errors (> -32000)"]
    end
```

## 7. Putting It All Together: Complete MCP System

Now let's see how all these LEGO pieces fit together to create a complete MCP system.

```mermaid
graph TD
    subgraph "Host Application (e.g., IDE)"
        ClientApp[Client Application]
        ClientProtocol[Protocol Layer]
        ClientTransport[Transport Layer]
        
        ClientApp --> ClientProtocol
        ClientProtocol --> ClientTransport
    end
    
    subgraph "Server Application"
        ServerTransport[Transport Layer]
        ServerProtocol[Protocol Layer]
        ServerApp[Server Application]
        ToolRegistry[Tool Registry]
        
        Tool1[Web Search]
        Tool2[Code Execution]
        Tool3[Database Query]
        
        ServerTransport --> ServerProtocol
        ServerProtocol --> ServerApp
        ServerApp --> ToolRegistry
        ToolRegistry --> Tool1
        ToolRegistry --> Tool2
        ToolRegistry --> Tool3
    end
    
    ClientTransport <--> |JSON-RPC 2.0| ServerTransport
    
    subgraph "External Services"
        WebAPI[Web API]
        CodeRunner[Code Runner]
        Database[Database]
        
        Tool1 --> WebAPI
        Tool2 --> CodeRunner
        Tool3 --> Database
    end
```

## 8. Implementation Example: Building Your Own MCP System

Let's walk through a simple example of building an MCP system, step by step.

### Step 1: Create the Server
```mermaid
graph TD
    Step1[Create Server] --> DefineInfo[Define Server Info]
    DefineInfo --> DefineCapabilities[Define Capabilities]
    DefineCapabilities --> RegisterTools[Register Tools]
    RegisterTools --> ImplementHandlers[Implement Tool Handlers]
    ImplementHandlers --> SelectTransport[Select Transport]
    SelectTransport --> StartServer[Start Server]
```

### Step 2: Create the Client
```mermaid
graph TD
    Step2[Create Client] --> SelectClientTransport[Select Transport]
    SelectClientTransport --> Connect[Connect to Server]
    Connect --> Initialize[Initialize Connection]
    Initialize --> DiscoverTools[Discover Available Tools]
    DiscoverTools --> CallTools[Call Tools]
    CallTools --> HandleResponses[Handle Responses]
    HandleResponses --> Shutdown[Shutdown When Done]
```

### Step 3: Complete Interaction Flow
```mermaid
sequenceDiagram
    participant User
    participant Client
    participant Server
    participant Tool
    participant ExternalService
    
    User->>Client: Request information
    Client->>Server: initialize request
    Server->>Client: initialize response
    Client->>Server: initialized notification
    
    Client->>Server: tool_call request
    Server->>Tool: Execute tool
    Tool->>ExternalService: API call
    ExternalService->>Tool: API response
    Tool->>Server: Tool result
    Server->>Client: tool_call response
    
    Client->>User: Display result
    
    Client->>Server: shutdown request
    Server->>Client: shutdown response
```

## 9. Security Considerations: Protecting Your LEGO Tower

Security in MCP is like making sure your LEGO tower is stable and protected.

```mermaid
graph TD
    Security[Security Considerations] --> Transport[Transport Security]
    Security --> Authentication[Authentication]
    Security --> Authorization[Authorization]
    Security --> Validation[Input Validation]
    Security --> ResourceProtection[Resource Protection]
    
    Transport --> TLS[Use TLS for Remote]
    Transport --> ValidateOrigin[Validate Connection Origins]
    
    Authentication --> ImplementAuth[Implement Authentication]
    Authentication --> SecureTokens[Secure Token Handling]
    
    Authorization --> AccessControl[Implement Access Controls]
    Authorization --> ResourcePermissions[Resource Permissions]
    
    Validation --> ValidateMessages[Validate All Messages]
    Validation --> SanitizeInputs[Sanitize Inputs]
    Validation --> SizeLimits[Message Size Limits]
    
    ResourceProtection --> RateLimiting[Rate Limiting]
    ResourceProtection --> Monitoring[Resource Monitoring]
    ResourceProtection --> Timeouts[Implement Timeouts]
```

## 10. Debugging and Monitoring: Maintaining Your LEGO Tower

Debugging and monitoring in MCP is like making sure your LEGO tower stays in good shape.

```mermaid
graph TD
    Debugging[Debugging & Monitoring] --> Logging[Logging]
    Debugging --> Diagnostics[Diagnostics]
    Debugging --> Testing[Testing]
    
    Logging --> LogProtocol[Log Protocol Events]
    Logging --> TrackMessages[Track Message Flow]
    Logging --> MonitorPerformance[Monitor Performance]
    Logging --> RecordErrors[Record Errors]
    
    Diagnostics --> HealthChecks[Implement Health Checks]
    Diagnostics --> ConnectionState[Monitor Connection State]
    Diagnostics --> ResourceUsage[Track Resource Usage]
    Diagnostics --> ProfilePerformance[Profile Performance]
    
    Testing --> TestTransports[Test Different Transports]
    Testing --> VerifyErrors[Verify Error Handling]
    Testing --> CheckEdgeCases[Check Edge Cases]
    Testing --> LoadTest[Load Test Servers]
```

## Conclusion

The Model Context Protocol provides a flexible, standardized way for AI assistants to communicate with tools and services. By understanding the building blocks of MCP - from the transport layer to message formats to the connection lifecycle - you can build powerful, interoperable AI systems.

Just like building with LEGO, MCP allows you to create complex structures from simple, well-defined pieces that fit together in predictable ways. This modularity and standardization enable innovation while ensuring compatibility across different implementations.

## References

- [MCP Core Architecture](https://modelcontextprotocol.io/docs/concepts/architecture)
- [MCP Transports](https://modelcontextprotocol.io/docs/concepts/transports) 