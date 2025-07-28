// PCF API Documentation Custom JavaScript

// Add copy buttons to code blocks
document.addEventListener('DOMContentLoaded', function() {
    // Add copy buttons to all code blocks
    const codeBlocks = document.querySelectorAll('pre code');
    codeBlocks.forEach(function(codeBlock) {
        const pre = codeBlock.parentElement;
        const button = document.createElement('button');
        button.className = 'copy-button';
        button.textContent = 'Copy';
        
        button.addEventListener('click', function() {
            const text = codeBlock.textContent;
            navigator.clipboard.writeText(text).then(function() {
                button.textContent = 'Copied!';
                setTimeout(function() {
                    button.textContent = 'Copy';
                }, 2000);
            }).catch(function(err) {
                console.error('Failed to copy:', err);
                button.textContent = 'Failed';
            });
        });
        
        pre.appendChild(button);
    });
    
    // Placeholder for interactive diagram functionality
    const diagrams = document.querySelectorAll('.interactive-diagram');
    diagrams.forEach(function(diagram) {
        console.log('Interactive diagram placeholder found:', diagram);
        // Future: Add click handlers, tooltips, etc.
    });
    
    // Placeholder for API playground functionality
    const playButtons = document.querySelectorAll('.play-button');
    playButtons.forEach(function(button) {
        button.addEventListener('click', function() {
            console.log('GraphQL playground execution (mock)');
            const responseViewer = button.closest('.playground-mock').querySelector('.response-viewer pre code');
            if (responseViewer) {
                responseViewer.textContent = JSON.stringify({
                    "data": {
                        "status": "This is a mock response. In production, this would execute against a real GraphQL endpoint."
                    }
                }, null, 2);
            }
        });
    });
    
    // Enhanced search functionality placeholder
    const searchInput = document.querySelector('#searchbar input');
    if (searchInput) {
        // Future: Add search analytics, suggestions, etc.
        searchInput.addEventListener('input', function(e) {
            console.log('Search query:', e.target.value);
        });
    }
    
    // Smooth scrolling for anchor links
    document.querySelectorAll('a[href^="#"]').forEach(anchor => {
        anchor.addEventListener('click', function (e) {
            e.preventDefault();
            const target = document.querySelector(this.getAttribute('href'));
            if (target) {
                target.scrollIntoView({
                    behavior: 'smooth',
                    block: 'start'
                });
            }
        });
    });
    
    // Add loading indicator for slow pages
    let loadingTimeout;
    window.addEventListener('beforeunload', function() {
        loadingTimeout = setTimeout(function() {
            document.body.classList.add('loading');
        }, 100);
    });
    
    window.addEventListener('load', function() {
        clearTimeout(loadingTimeout);
        document.body.classList.remove('loading');
    });
    
    // Keyboard shortcuts enhancement
    document.addEventListener('keydown', function(e) {
        // Ctrl/Cmd + K for search
        if ((e.ctrlKey || e.metaKey) && e.key === 'k') {
            e.preventDefault();
            const searchInput = document.querySelector('#searchbar input');
            if (searchInput) {
                searchInput.focus();
            }
        }
    });
    
    // Performance timing (development helper)
    if (window.performance && window.performance.timing) {
        window.addEventListener('load', function() {
            setTimeout(function() {
                const timing = window.performance.timing;
                const loadTime = timing.loadEventEnd - timing.navigationStart;
                console.log('Page load time:', loadTime + 'ms');
                
                // Future: Send to analytics
                if (loadTime > 3000) {
                    console.warn('Page load time exceeds 3 seconds');
                }
            }, 0);
        });
    }
});

// Placeholder for future analytics integration
window.PCFDocs = {
    trackEvent: function(category, action, label) {
        console.log('Analytics event:', { category, action, label });
        // Future: Send to analytics service
    },
    
    reportError: function(error) {
        console.error('Documentation error:', error);
        // Future: Send to error tracking service
    }
};