# OPA Policy
package authz

default allow = false

# Extract tenant from JWT
tenant = payload.tenant {
    [_, encoded] := io.jwt.decode_verify(input.attributes.request.http.headers.authorization, {"secret": "your-secret"})
    payload := encoded
}

# Service-to-service: Allow if same tenant
allow {
    input.attributes.source.workload.name == "frontend"
    input.attributes.destination.workload.name == "api"
    tenant == input.attributes.destination.labels["tenant"]
}

# User permissions: Check role-based access
allow {
    input.attributes.request.http.method == "GET"
    input.attributes.request.http.path == ["api", "v1", "users", user_id]
    payload.sub == user_id  # Users can only access their own data
}

allow {
    input.attributes.request.http.method == "GET"
    input.attributes.request.http.path == ["api", "v1", "users", _]
    payload.role == "admin"  # Admins can access any user
}
