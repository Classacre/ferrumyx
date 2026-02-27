document.addEventListener('DOMContentLoaded', () => {
    // 1. Theme Manager
    const themeParams = {
        LIGHT: 'light',
        DARK: 'dark',
        SYSTEM: 'system'
    };

    function applyTheme(theme) {
        if (theme === themeParams.DARK) {
            document.documentElement.setAttribute('data-theme', 'dark');
        } else if (theme === themeParams.LIGHT) {
            document.documentElement.setAttribute('data-theme', 'light');
        } else {
            // System: remove manual override
            document.documentElement.removeAttribute('data-theme');
        }
    }

    // Load saved or default theme
    const savedTheme = localStorage.getItem('ferrumyx-theme') || themeParams.SYSTEM;
    applyTheme(savedTheme);

    // Provide toggle globally for Settings page
    window.setTheme = function(theme) {
        localStorage.setItem('ferrumyx-theme', theme);
        applyTheme(theme);
        updateThemeUI(theme);
    };

    function updateThemeUI(theme) {
        // Update active class on settings buttons if they exist
        const buttons = document.querySelectorAll('.theme-btn');
        if (buttons.length > 0) {
            buttons.forEach(btn => btn.classList.remove('active', 'btn-primary'));
            buttons.forEach(btn => btn.classList.add('btn-outline-secondary'));
            
            const activeBtn = document.getElementById(`theme-btn-${theme}`);
            if (activeBtn) {
                activeBtn.classList.remove('btn-outline-secondary');
                activeBtn.classList.add('active', 'btn-primary');
            }
        }
    }

    // Call it once to set initial active button state
    updateThemeUI(savedTheme);

    // 2. Active Nav Link Manager
    const path = window.location.pathname;
    const links = document.querySelectorAll('.nav-links a');
    links.forEach(link => {
        if (path === '/' && link.getAttribute('href') === '/') {
            link.classList.add('active');
        } else if (path !== '/' && link.getAttribute('href').startsWith(path) && link.getAttribute('href') !== '/') {
            link.classList.add('active');
        }
    });

});
