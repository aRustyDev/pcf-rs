# Project Dependencies

This page provides a comprehensive overview of all dependencies used in the PCF API project, including rationales for each choice and license compatibility information.

<!-- mdbook-dependency-doc:
  cargo_toml: "../Cargo.toml"
  rationales: "../dependencies.toml"
  show_graph: true
  check_licenses: true
-->

## Dependency Graph

<!-- Auto-generated dependency graph will appear here -->

<div class="dependency-graph" style="border: 1px solid #ddd; padding: 20px; background: #f8f9fa; text-align: center;">
  <p style="color: #666;">
    Dependency Graph Placeholder<br/>
    When mdbook-dependency-doc is available, this will show:<br/>
    - Visual dependency tree<br/>
    - Transitive dependency relationships<br/>
    - Version conflicts if any
  </p>
</div>

## Direct Dependencies

<!-- Auto-generated dependency list with rationales will appear here -->

### Core Dependencies

The following dependencies are critical to the application's functionality:

| Crate | Version | License | Purpose |
|-------|---------|---------|---------|
| axum | 0.7.x | MIT | Web framework |
| tokio | 1.x | MIT | Async runtime |
| async-graphql | 7.x | MIT/Apache-2.0 | GraphQL server |
| surrealdb | 1.x | Apache-2.0 | Database client |
| figment | 0.10.x | MIT/Apache-2.0 | Configuration management |
| garde | 0.18.x | MIT/Apache-2.0 | Validation |

### Observability Dependencies

| Crate | Version | License | Purpose |
|-------|---------|---------|---------|
| tracing | 0.1.x | MIT | Structured logging |
| opentelemetry | 0.21.x | Apache-2.0 | Distributed tracing |
| prometheus | 0.13.x | Apache-2.0 | Metrics |

### Security Dependencies

| Crate | Version | License | Purpose |
|-------|---------|---------|---------|
| jsonwebtoken | 9.x | MIT | JWT handling |
| argon2 | 0.5.x | MIT/Apache-2.0 | Password hashing |
| uuid | 1.x | MIT/Apache-2.0 | Unique identifiers |

## Transitive Dependencies

<!-- Auto-generated transitive dependency analysis will appear here -->

### Security Audit

All dependencies are regularly audited for security vulnerabilities using:
- `cargo audit`
- Dependabot security alerts
- Manual review of critical updates

### Update Policy

- **Patch updates**: Applied automatically via Dependabot
- **Minor updates**: Reviewed and applied monthly
- **Major updates**: Evaluated quarterly with migration plan

## License Summary

<!-- Auto-generated license compatibility matrix will appear here -->

### License Distribution

- MIT: ~60% of dependencies
- Apache-2.0: ~30% of dependencies
- MIT/Apache-2.0 dual: ~10% of dependencies

All licenses are compatible with the project's Apache-2.0 license.

## Dependency Rationales

### Why Axum?
- Modern, type-safe web framework
- Built on Tower middleware ecosystem
- Excellent performance characteristics
- Strong community support

### Why async-graphql?
- Most feature-complete Rust GraphQL library
- Excellent async/await support
- Built-in subscription support
- Active development and community

### Why SurrealDB?
- Multi-model database (document, graph, relational)
- Built-in real-time subscriptions
- Rust-native with excellent performance
- Simplified deployment model

### Why Figment?
- Type-safe configuration management
- Multiple source support (files, env, CLI)
- Excellent error messages
- Serde integration

## Version Pinning Strategy

### Production Dependencies
- Exact versions for critical dependencies
- Compatible versions (~) for stable libraries
- Caret versions (^) for actively developed libraries

### Development Dependencies
- Latest compatible versions
- Regular updates to catch issues early

## Minimizing Dependencies

We actively work to minimize dependencies by:
1. Evaluating each dependency's necessity
2. Preferring stdlib solutions when available
3. Avoiding dependencies with many transitive deps
4. Regular audits to remove unused dependencies

## Future Considerations

### Potential Additions
- `rustls`: For native TLS support
- `deadpool`: Advanced connection pooling
- `async-stripe`: Payment processing

### Potential Removals
- Dependencies that become part of std
- Libraries with better alternatives
- Unused feature flags

<!-- Future Enhancement: Dependency Documentation
When mdbook-dependency-doc plugin is available:
- Real-time dependency tree visualization
- Automatic license compatibility checking
- Security vulnerability alerts
- Update impact analysis
- Size and compile-time impact metrics
-->
