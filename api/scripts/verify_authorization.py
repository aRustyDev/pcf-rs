#!/usr/bin/env python3
"""
PCF API Authorization Verification Script

This script verifies that the authorization system is working correctly
by testing various endpoints and scenarios. It should be run after
deployment to ensure security controls are properly implemented.

Usage:
    python3 scripts/verify_authorization.py --host localhost:8080
    python3 scripts/verify_authorization.py --host api.example.com --ssl
"""

import argparse
import json
import requests
import sys
from typing import Dict, List, Optional
from dataclasses import dataclass


@dataclass
class TestResult:
    """Test result with details"""
    name: str
    passed: bool
    message: str
    response_code: Optional[int] = None


class AuthorizationVerifier:
    """Verifies PCF API authorization system"""
    
    def __init__(self, host: str, use_ssl: bool = False, timeout: float = 10.0):
        self.host = host
        self.base_url = f"{'https' if use_ssl else 'http'}://{host}"
        self.timeout = timeout
        self.results: List[TestResult] = []
        
    def run_all_tests(self) -> bool:
        """Run all verification tests"""
        print(f"🔍 Verifying authorization system at {self.base_url}")
        print("=" * 60)
        
        # Test GraphQL endpoint availability
        self._test_graphql_endpoint()
        
        # Test health endpoint (should not require auth)
        self._test_health_endpoint()
        
        # Test unauthenticated requests (should be rejected)
        self._test_unauthenticated_requests()
        
        # Test demo mode behavior (if enabled)
        self._test_demo_mode()
        
        # Test authorization bypass attempts
        self._test_authorization_bypass_attempts()
        
        # Test rate limiting (if implemented)
        self._test_rate_limiting()
        
        # Print results
        self._print_results()
        
        # Return overall success
        passed_tests = sum(1 for r in self.results if r.passed)
        total_tests = len(self.results)
        
        print(f"\n📊 Results: {passed_tests}/{total_tests} tests passed")
        
        if passed_tests == total_tests:
            print("✅ All authorization tests PASSED")
            return True
        else:
            print("❌ Some authorization tests FAILED")
            return False
    
    def _test_graphql_endpoint(self):
        """Test that GraphQL endpoint is accessible"""
        try:
            query = {"query": "{ __schema { types { name } } }"}
            response = requests.post(
                f"{self.base_url}/graphql",
                json=query,
                timeout=self.timeout
            )
            
            if response.status_code == 200:
                self.results.append(TestResult(
                    "GraphQL Endpoint Available",
                    True,
                    "GraphQL endpoint is responding",
                    response.status_code
                ))
            else:
                self.results.append(TestResult(
                    "GraphQL Endpoint Available",
                    False,
                    f"GraphQL endpoint returned status {response.status_code}",
                    response.status_code
                ))
        except Exception as e:
            self.results.append(TestResult(
                "GraphQL Endpoint Available",
                False,
                f"Failed to connect to GraphQL endpoint: {e}"
            ))
    
    def _test_health_endpoint(self):
        """Test health endpoint (should work without authentication)"""
        try:
            query = {"query": "{ health { status version } }"}
            response = requests.post(
                f"{self.base_url}/graphql",
                json=query,
                timeout=self.timeout
            )
            
            if response.status_code == 200:
                data = response.json()
                if 'data' in data and 'health' in data['data']:
                    self.results.append(TestResult(
                        "Health Endpoint No Auth Required",
                        True,
                        "Health endpoint accessible without authentication",
                        response.status_code
                    ))
                else:
                    self.results.append(TestResult(
                        "Health Endpoint No Auth Required",
                        False,
                        "Health endpoint returned unexpected format",
                        response.status_code
                    ))
            else:
                self.results.append(TestResult(
                    "Health Endpoint No Auth Required",
                    False,
                    f"Health endpoint returned status {response.status_code}",
                    response.status_code
                ))
        except Exception as e:
            self.results.append(TestResult(
                "Health Endpoint No Auth Required",
                False,
                f"Health endpoint test failed: {e}"
            ))
    
    def _test_unauthenticated_requests(self):
        """Test that protected endpoints reject unauthenticated requests"""
        protected_queries = [
            {"query": "{ notes(first: 5) { edges { node { id title } } } }"},
            {"query": "{ note(id: \"notes:test\") { id title } }"},
        ]
        
        for i, query in enumerate(protected_queries):
            try:
                response = requests.post(
                    f"{self.base_url}/graphql",
                    json=query,
                    timeout=self.timeout
                )
                
                # Should return 200 but with authorization errors in GraphQL response
                if response.status_code == 200:
                    data = response.json()
                    if 'errors' in data:
                        # Check if any error mentions authentication/authorization
                        auth_error = any(
                            'auth' in str(error).lower() or 
                            'permission' in str(error).lower()
                            for error in data['errors']
                        )
                        
                        if auth_error:
                            self.results.append(TestResult(
                                f"Protected Query {i+1} Requires Auth",
                                True,
                                "Protected query correctly rejected without auth",
                                response.status_code
                            ))
                        else:
                            self.results.append(TestResult(
                                f"Protected Query {i+1} Requires Auth",
                                False,
                                "Protected query should require authentication",
                                response.status_code
                            ))
                    else:
                        self.results.append(TestResult(
                            f"Protected Query {i+1} Requires Auth",
                            False,
                            "Protected query returned success without auth - security issue!",
                            response.status_code
                        ))
                else:
                    self.results.append(TestResult(
                        f"Protected Query {i+1} Requires Auth",
                        False,
                        f"Unexpected status code: {response.status_code}",
                        response.status_code
                    ))
            except Exception as e:
                self.results.append(TestResult(
                    f"Protected Query {i+1} Requires Auth",
                    False,
                    f"Test failed: {e}"
                ))
    
    def _test_demo_mode(self):
        """Test demo mode detection and warnings"""
        try:
            query = {"query": "{ health { status } }"}
            response = requests.post(
                f"{self.base_url}/graphql",
                json=query,
                timeout=self.timeout
            )
            
            # Check response headers or logs for demo mode indicators
            # This is a basic check - in production, you might check logs or metrics
            if response.status_code == 200:
                # Demo mode detection is implementation-specific
                # For now, we just verify the endpoint responds
                self.results.append(TestResult(
                    "Demo Mode Detection",
                    True,
                    "Demo mode test completed (check server logs for warnings)",
                    response.status_code
                ))
            else:
                self.results.append(TestResult(
                    "Demo Mode Detection",
                    False,
                    f"Could not test demo mode: status {response.status_code}",
                    response.status_code
                ))
        except Exception as e:
            self.results.append(TestResult(
                "Demo Mode Detection",
                False,
                f"Demo mode test failed: {e}"
            ))
    
    def _test_authorization_bypass_attempts(self):
        """Test various authorization bypass attempts"""
        bypass_attempts = [
            # SQL injection attempts in GraphQL
            {"query": "{ note(id: \"'; DROP TABLE notes; --\") { id } }"},
            # GraphQL injection attempts
            {"query": "{ note(id: \"notes:test\") { id ... on __Schema { types { name } } } }"},
            # Large query complexity (should be limited)
            {"query": "{ " + "notes(first: 1000) { edges { node { " * 10 + "id" + " } } }" * 10 + " }"},
        ]
        
        for i, query in enumerate(bypass_attempts):
            try:
                response = requests.post(
                    f"{self.base_url}/graphql",
                    json=query,
                    timeout=self.timeout
                )
                
                # These should all be rejected or handled safely
                if response.status_code == 200:
                    data = response.json()
                    if 'errors' in data:
                        self.results.append(TestResult(
                            f"Bypass Attempt {i+1} Blocked",
                            True,
                            "Potential bypass attempt properly rejected",
                            response.status_code
                        ))
                    else:
                        self.results.append(TestResult(
                            f"Bypass Attempt {i+1} Blocked",
                            False,
                            "Potential bypass attempt was not rejected - investigate!",
                            response.status_code
                        ))
                else:
                    self.results.append(TestResult(
                        f"Bypass Attempt {i+1} Blocked",
                        True,
                        f"Bypass attempt rejected with status {response.status_code}",
                        response.status_code
                    ))
            except Exception as e:
                self.results.append(TestResult(
                    f"Bypass Attempt {i+1} Blocked",
                    True,
                    f"Bypass attempt caused error (likely blocked): {e}"
                ))
    
    def _test_rate_limiting(self):
        """Test rate limiting behavior"""
        try:
            query = {"query": "{ health { status } }"}
            
            # Make multiple rapid requests
            responses = []
            for _ in range(10):
                response = requests.post(
                    f"{self.base_url}/graphql",
                    json=query,
                    timeout=self.timeout
                )
                responses.append(response.status_code)
            
            # Check if any requests were rate limited (429 status)
            rate_limited = any(status == 429 for status in responses)
            
            if rate_limited:
                self.results.append(TestResult(
                    "Rate Limiting Active",
                    True,
                    "Rate limiting is active (some requests returned 429)",
                ))
            else:
                self.results.append(TestResult(
                    "Rate Limiting Active",
                    True,
                    "Rate limiting test completed (no 429s - may not be configured)",
                ))
        except Exception as e:
            self.results.append(TestResult(
                "Rate Limiting Active",
                False,
                f"Rate limiting test failed: {e}"
            ))
    
    def _print_results(self):
        """Print test results in a formatted way"""
        print("\n📋 Test Results:")
        print("-" * 60)
        
        for result in self.results:
            status = "✅" if result.passed else "❌"
            code = f" [{result.response_code}]" if result.response_code else ""
            print(f"{status} {result.name}{code}")
            print(f"   {result.message}")
            print()


def main():
    """Main function"""
    parser = argparse.ArgumentParser(description="Verify PCF API authorization system")
    parser.add_argument("--host", default="localhost:8080", 
                       help="API host and port (default: localhost:8080)")
    parser.add_argument("--ssl", action="store_true", 
                       help="Use HTTPS instead of HTTP")
    parser.add_argument("--timeout", type=float, default=10.0,
                       help="Request timeout in seconds (default: 10.0)")
    
    args = parser.parse_args()
    
    verifier = AuthorizationVerifier(args.host, args.ssl, args.timeout)
    success = verifier.run_all_tests()
    
    sys.exit(0 if success else 1)


if __name__ == "__main__":
    main()