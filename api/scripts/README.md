# PCF API Scripts

This directory contains utility scripts for the PCF API.

## Authorization Verification

### verify-auth.sh

A comprehensive shell script that verifies the authorization system is working correctly.

**Usage:**
```bash
# Basic usage (localhost)
./scripts/verify-auth.sh

# Test remote API
./scripts/verify-auth.sh --host api.example.com --ssl

# Custom timeout
./scripts/verify-auth.sh --host localhost:3000 --timeout 60
```

**Options:**
- `--host HOST`: API host and port (default: localhost:8080)
- `--ssl`: Use HTTPS instead of HTTP
- `--timeout SEC`: Request timeout in seconds (default: 30)
- `--help, -h`: Show help message

### verify_authorization.py

A Python script that performs detailed authorization testing.

**Requirements:**
- Python 3.6+
- `requests` library (`pip install requests`)

**Usage:**
```bash
# Basic usage
python3 scripts/verify_authorization.py

# Test with SSL
python3 scripts/verify_authorization.py --host api.example.com --ssl

# Custom timeout
python3 scripts/verify_authorization.py --timeout 30.0
```

## What These Scripts Test

1. **Basic Connectivity**: Verifies the API is reachable
2. **GraphQL Endpoint**: Confirms GraphQL is responding correctly
3. **Health Endpoint**: Tests that health checks work without authentication
4. **Authorization Requirements**: Verifies protected endpoints require authentication
5. **Demo Mode Detection**: Checks for demo mode indicators
6. **Security Bypass Attempts**: Tests various attack vectors
7. **Rate Limiting**: Verifies rate limiting behavior

## Security Checklist

After running these scripts, manually verify:

- [ ] HTTPS is used in production
- [ ] Demo mode is disabled in production
- [ ] Rate limiting is configured appropriately
- [ ] Authorization audit logs are being generated
- [ ] SpiceDB connection is secure
- [ ] Test with actual user credentials

## Integration with CI/CD

These scripts can be integrated into your deployment pipeline:

```yaml
# Example GitHub Actions step
- name: Verify Authorization
  run: |
    ./scripts/verify-auth.sh --host ${{ env.API_HOST }} --ssl
```

## Troubleshooting

### Common Issues

1. **Connection Refused**: Make sure the API server is running
2. **SSL Certificate Errors**: Use `--ssl` only for HTTPS endpoints
3. **Python Dependencies**: Install with `pip install requests`
4. **Permission Denied**: Make sure scripts are executable (`chmod +x`)

### Expected Results

- Health endpoint should work without authentication
- Protected endpoints should require authentication
- No obvious security bypasses should work
- Demo mode should be disabled in production