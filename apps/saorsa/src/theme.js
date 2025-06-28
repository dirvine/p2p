// Theme Management System
export class ThemeManager {
    constructor() {
        this.themes = {
            light: {
                name: 'Light',
                colors: {
                    // Background colors
                    bgPrimary: '#FFFFFF',
                    bgSecondary: '#F9FAFB',
                    bgTertiary: '#F3F4F6',
                    bgHover: '#E5E7EB',
                    
                    // Text colors
                    textPrimary: '#111827',
                    textSecondary: '#4B5563',
                    textTertiary: '#6B7280',
                    textInverse: '#FFFFFF',
                    
                    // Border colors
                    borderPrimary: '#E5E7EB',
                    borderSecondary: '#D1D5DB',
                    borderFocus: '#3B82F6',
                    
                    // Component specific
                    navBg: '#FFFFFF',
                    sidebarBg: '#F9FAFB',
                    messageBg: '#FFFFFF',
                    messageHover: '#F3F4F6',
                    composerBg: '#FFFFFF',
                    
                    // Status colors
                    online: '#10B981',
                    away: '#F59E0B',
                    offline: '#6B7280',
                    
                    // Shadows
                    shadowColor: 'rgba(0, 0, 0, 0.1)',
                    shadowColorLight: 'rgba(0, 0, 0, 0.05)'
                }
            },
            dark: {
                name: 'Dark',
                colors: {
                    // Background colors
                    bgPrimary: '#0F172A',
                    bgSecondary: '#1E293B',
                    bgTertiary: '#334155',
                    bgHover: '#475569',
                    
                    // Text colors
                    textPrimary: '#F9FAFB',
                    textSecondary: '#CBD5E1',
                    textTertiary: '#94A3B8',
                    textInverse: '#0F172A',
                    
                    // Border colors
                    borderPrimary: '#334155',
                    borderSecondary: '#475569',
                    borderFocus: '#3B82F6',
                    
                    // Component specific
                    navBg: '#1E293B',
                    sidebarBg: '#0F172A',
                    messageBg: '#1E293B',
                    messageHover: '#334155',
                    composerBg: '#1E293B',
                    
                    // Status colors
                    online: '#10B981',
                    away: '#F59E0B',
                    offline: '#64748B',
                    
                    // Shadows
                    shadowColor: 'rgba(0, 0, 0, 0.3)',
                    shadowColorLight: 'rgba(0, 0, 0, 0.2)'
                }
            }
        };
        
        this.currentTheme = 'system';
        this.systemTheme = this.getSystemTheme();
        this.init();
    }
    
    init() {
        // Load saved theme preference
        const savedTheme = localStorage.getItem('theme') || 'system';
        this.setTheme(savedTheme);
        
        // Listen for system theme changes
        if (window.matchMedia) {
            const darkModeQuery = window.matchMedia('(prefers-color-scheme: dark)');
            darkModeQuery.addEventListener('change', (e) => {
                this.systemTheme = e.matches ? 'dark' : 'light';
                if (this.currentTheme === 'system') {
                    this.applyTheme(this.systemTheme);
                }
            });
        }
    }
    
    getSystemTheme() {
        if (window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches) {
            return 'dark';
        }
        return 'light';
    }
    
    setTheme(theme) {
        this.currentTheme = theme;
        localStorage.setItem('theme', theme);
        
        const effectiveTheme = theme === 'system' ? this.systemTheme : theme;
        this.applyTheme(effectiveTheme);
        
        // Dispatch theme change event
        window.dispatchEvent(new CustomEvent('themechange', { 
            detail: { theme: effectiveTheme, mode: theme } 
        }));
    }
    
    applyTheme(themeName) {
        const theme = this.themes[themeName];
        const root = document.documentElement;
        
        // Apply theme class
        root.className = `theme-${themeName}`;
        
        // Apply CSS variables
        Object.entries(theme.colors).forEach(([key, value]) => {
            root.style.setProperty(`--theme-${this.kebabCase(key)}`, value);
        });
        
        // Update meta theme-color
        const metaThemeColor = document.querySelector('meta[name="theme-color"]');
        if (metaThemeColor) {
            metaThemeColor.content = theme.colors.navBg;
        } else {
            const meta = document.createElement('meta');
            meta.name = 'theme-color';
            meta.content = theme.colors.navBg;
            document.head.appendChild(meta);
        }
    }
    
    kebabCase(str) {
        return str.replace(/([a-z])([A-Z])/g, '$1-$2').toLowerCase();
    }
    
    getCurrentTheme() {
        return this.currentTheme;
    }
    
    getEffectiveTheme() {
        return this.currentTheme === 'system' ? this.systemTheme : this.currentTheme;
    }
    
    getThemeColors() {
        const effectiveTheme = this.getEffectiveTheme();
        return this.themes[effectiveTheme].colors;
    }
}

// Settings UI Component
export class ThemeSettings {
    constructor(themeManager) {
        this.themeManager = themeManager;
    }
    
    render() {
        const currentTheme = this.themeManager.getCurrentTheme();
        
        return `
            <div class="settings-section">
                <h3 class="settings-title">Appearance</h3>
                <div class="settings-group">
                    <label class="settings-label">Theme</label>
                    <div class="theme-selector">
                        <button class="theme-option ${currentTheme === 'system' ? 'active' : ''}" data-theme="system">
                            <svg class="theme-icon" width="24" height="24" viewBox="0 0 24 24">
                                <path d="M12 2.25a.75.75 0 01.75.75v2.25a.75.75 0 01-1.5 0V3a.75.75 0 01.75-.75zM7.5 12a4.5 4.5 0 119 0 4.5 4.5 0 01-9 0zM18.894 6.166a.75.75 0 00-1.06-1.06l-1.591 1.59a.75.75 0 101.06 1.061l1.591-1.59zM21.75 12a.75.75 0 01-.75.75h-2.25a.75.75 0 010-1.5H21a.75.75 0 01.75.75zM17.834 18.894a.75.75 0 001.06-1.06l-1.59-1.591a.75.75 0 10-1.061 1.06l1.59 1.591zM12 18a.75.75 0 01.75.75V21a.75.75 0 01-1.5 0v-2.25A.75.75 0 0112 18zM7.758 17.303a.75.75 0 00-1.061-1.06l-1.591 1.59a.75.75 0 001.06 1.061l1.591-1.59zM6 12a.75.75 0 01-.75.75H3a.75.75 0 010-1.5h2.25A.75.75 0 016 12zM6.697 7.757a.75.75 0 001.06-1.06l-1.59-1.591a.75.75 0 00-1.061 1.06l1.59 1.591z" fill="currentColor"/>
                            </svg>
                            <span>System</span>
                            <p class="theme-description">Use your system preference</p>
                        </button>
                        
                        <button class="theme-option ${currentTheme === 'light' ? 'active' : ''}" data-theme="light">
                            <svg class="theme-icon" width="24" height="24" viewBox="0 0 24 24">
                                <path d="M12 2.25a.75.75 0 01.75.75v2.25a.75.75 0 01-1.5 0V3a.75.75 0 01.75-.75zM7.5 12a4.5 4.5 0 119 0 4.5 4.5 0 01-9 0zM18.894 6.166a.75.75 0 00-1.06-1.06l-1.591 1.59a.75.75 0 101.06 1.061l1.591-1.59zM21.75 12a.75.75 0 01-.75.75h-2.25a.75.75 0 010-1.5H21a.75.75 0 01.75.75zM17.834 18.894a.75.75 0 001.06-1.06l-1.59-1.591a.75.75 0 10-1.061 1.06l1.59 1.591zM12 18a.75.75 0 01.75.75V21a.75.75 0 01-1.5 0v-2.25A.75.75 0 0112 18zM7.758 17.303a.75.75 0 00-1.061-1.06l-1.591 1.59a.75.75 0 001.06 1.061l1.591-1.59zM6 12a.75.75 0 01-.75.75H3a.75.75 0 010-1.5h2.25A.75.75 0 016 12zM6.697 7.757a.75.75 0 001.06-1.06l-1.59-1.591a.75.75 0 00-1.061 1.06l1.59 1.591z" fill="currentColor"/>
                            </svg>
                            <span>Light</span>
                            <p class="theme-description">Light background with dark text</p>
                        </button>
                        
                        <button class="theme-option ${currentTheme === 'dark' ? 'active' : ''}" data-theme="dark">
                            <svg class="theme-icon" width="24" height="24" viewBox="0 0 24 24">
                                <path d="M9.528 1.718a.75.75 0 01.162.819A8.97 8.97 0 009 6a9 9 0 009 9 8.97 8.97 0 003.463-.69.75.75 0 01.981.98 10.503 10.503 0 01-9.694 6.46c-5.799 0-10.5-4.701-10.5-10.5 0-4.368 2.667-8.112 6.46-9.694a.75.75 0 01.818.162z" fill="currentColor"/>
                            </svg>
                            <span>Dark</span>
                            <p class="theme-description">Dark background with light text</p>
                        </button>
                    </div>
                </div>
            </div>
        `;
    }
    
    attachEventListeners(container) {
        const themeOptions = container.querySelectorAll('.theme-option');
        
        themeOptions.forEach(option => {
            option.addEventListener('click', () => {
                const theme = option.dataset.theme;
                this.themeManager.setTheme(theme);
                
                // Update active state
                themeOptions.forEach(opt => opt.classList.remove('active'));
                option.classList.add('active');
            });
        });
    }
}

// Initialize theme manager
export const themeManager = new ThemeManager();