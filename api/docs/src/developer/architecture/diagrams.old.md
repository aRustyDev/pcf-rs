# Architecture Diagrams

Interactive architecture diagrams for the PCF API.

<!-- toc -->

## System Overview

<!-- mdbook-interactive-diagrams: id=system-overview, collapsible=true -->
```mermaid
graph TB
    subgraph "Client Layer"
        Web[Web App]
        CLI[CLI Tool]
        Mobile[Mobile App]
    end
    
    subgraph "API Gateway"
        LB[Load Balancer]
        RateLimit[Rate Limiter]
        Auth[Auth Service]
    end
    
    subgraph "API Layer"
        GraphQL[GraphQL Server]
        REST[REST Endpoints]
        WS[WebSocket Handler]
    end
    
    subgraph "Service Layer"
        UserSvc[User Service]
        ResourceSvc[Resource Service]
        NotifySvc[Notification Service]
    end
    
    subgraph "Data Layer"
        DB[(Primary DB)]
        Cache[(Redis Cache)]
        Queue[(Message Queue)]
    end
    
    Web --> LB
    CLI --> LB
    Mobile --> LB
    
    LB --> RateLimit
    RateLimit --> Auth
    Auth --> GraphQL
    Auth --> REST
    Auth --> WS
    
    GraphQL --> UserSvc
    GraphQL --> ResourceSvc
    REST --> UserSvc
    REST --> ResourceSvc
    WS --> NotifySvc
    
    UserSvc --> DB
    UserSvc --> Cache
    ResourceSvc --> DB
    ResourceSvc --> Cache
    NotifySvc --> Queue
```

## Request Flow

<!-- mdbook-interactive-diagrams: id=request-flow, animated=true -->
```mermaid
sequenceDiagram
    participant Client
    participant Gateway
    participant Auth
    participant GraphQL
    participant Service
    participant DB
    
    Client->>Gateway: HTTP Request
    Gateway->>Gateway: Rate Limit Check
    Gateway->>Auth: Validate Token
    Auth-->>Gateway: Token Valid
    Gateway->>GraphQL: Forward Request
    GraphQL->>GraphQL: Parse Query
    GraphQL->>GraphQL: Validate Query
    GraphQL->>Service: Execute Resolver
    Service->>DB: Query Data
    DB-->>Service: Return Data
    Service-->>GraphQL: Return Result
    GraphQL-->>Gateway: GraphQL Response
    Gateway-->>Client: HTTP Response
```

## Module Dependencies

<!-- mdbook-interactive-diagrams: id=module-deps, interactive=true -->
```mermaid
graph LR
    subgraph "Core Modules"
        Config[config]
        Errors[errors]
        Utils[utils]
    end
    
    subgraph "API Modules"
        GraphQLMod[graphql]
        REST[rest]
        Middleware[middleware]
    end
    
    subgraph "Service Modules"
        Auth[auth]
        Users[users]
        Resources[resources]
    end
    
    subgraph "Infrastructure"
        DB[database]
        Cache[cache]
        Telemetry[telemetry]
    end
    
    GraphQLMod --> Config
    GraphQLMod --> Errors
    GraphQLMod --> Auth
    GraphQLMod --> Users
    GraphQLMod --> Resources
    
    REST --> Config
    REST --> Errors
    REST --> Middleware
    
    Auth --> DB
    Auth --> Cache
    Users --> DB
    Resources --> DB
    
    Middleware --> Telemetry
    Middleware --> Config
```

## Database Schema

<!-- mdbook-interactive-diagrams: id=db-schema, expandable=true -->
```mermaid
erDiagram
    USER ||--o{ SESSION : has
    USER ||--o{ RESOURCE : owns
    USER ||--o{ PERMISSION : has
    RESOURCE ||--o{ RESOURCE_VERSION : has
    ROLE ||--o{ PERMISSION : contains
    USER ||--o{ ROLE : has
    
    USER {
        uuid id PK
        string email UK
        string name
        string password_hash
        timestamp created_at
        timestamp updated_at
    }
    
    SESSION {
        uuid id PK
        uuid user_id FK
        string token UK
        timestamp expires_at
        timestamp created_at
    }
    
    RESOURCE {
        uuid id PK
        uuid owner_id FK
        string name
        string type
        json metadata
        timestamp created_at
        timestamp updated_at
    }
    
    RESOURCE_VERSION {
        uuid id PK
        uuid resource_id FK
        int version
        json data
        uuid created_by FK
        timestamp created_at
    }
    
    ROLE {
        uuid id PK
        string name UK
        string description
        timestamp created_at
    }
    
    PERMISSION {
        uuid id PK
        string resource_type
        string action
        json conditions
    }
```

## Deployment Architecture

<!-- mdbook-interactive-diagrams: id=deployment, zoomable=true -->
```mermaid
graph TB
    subgraph "Kubernetes Cluster"
        subgraph "Ingress"
            Nginx[Nginx Ingress]
        end
        
        subgraph "API Pods"
            API1[API Pod 1]
            API2[API Pod 2]
            API3[API Pod 3]
        end
        
        subgraph "Service Pods"
            Auth1[Auth Service]
            Auth2[Auth Service]
        end
        
        subgraph "Data Services"
            PG[PostgreSQL]
            Redis[Redis]
            Kafka[Kafka]
        end
    end
    
    subgraph "External Services"
        S3[S3 Storage]
        Email[Email Service]
        Monitoring[Monitoring]
    end
    
    Nginx --> API1
    Nginx --> API2
    Nginx --> API3
    
    API1 --> Auth1
    API2 --> Auth1
    API3 --> Auth2
    
    API1 --> PG
    API2 --> PG
    API3 --> PG
    
    API1 --> Redis
    API2 --> Redis
    API3 --> Redis
    
    API1 --> Kafka
    Auth1 --> Redis
    Auth2 --> Redis
    
    API1 --> S3
    API2 --> Email
    API3 --> Monitoring
```

## Performance Metrics Flow

<!-- mdbook-interactive-diagrams: id=metrics-flow -->
```mermaid
graph LR
    App[Application] --> OTel[OpenTelemetry Collector]
    OTel --> Prometheus[Prometheus]
    OTel --> Jaeger[Jaeger]
    OTel --> Loki[Loki]
    
    Prometheus --> Grafana[Grafana]
    Jaeger --> Grafana
    Loki --> Grafana
    
    Grafana --> Alerts[Alert Manager]
    Alerts --> Slack[Slack]
    Alerts --> PagerDuty[PagerDuty]
```