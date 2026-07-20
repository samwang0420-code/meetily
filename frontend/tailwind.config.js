/** @type {import('tailwindcss').Config} */
module.exports = {
    darkMode: ['class'],
    content: [
    './src/pages/**/*.{js,ts,jsx,tsx,mdx}',
    './src/components/**/*.{js,ts,jsx,tsx,mdx}',
    './src/app/**/*.{js,ts,jsx,tsx,mdx}',
  ],
  theme: {
  	extend: {
  		fontFamily: {
  			sans: [
  				'"Inter"',
  				'"Geist"',
  				'-apple-system',
  				'BlinkMacSystemFont',
  				'"PingFang SC"',
  				'"Hiragino Sans GB"',
  				'sans-serif',
  				'var(--font-source-sans-3)'
  			],
  			mono: [
  				'"JetBrains Mono"',
  				'"Geist Mono"',
  				'ui-monospace',
  				'"SF Mono"',
  				'monospace'
  			],
  			serif: [
  				'"Copernicus"',
  				'"Tiempos Headline"',
  				'Georgia',
  				'serif'
  			]
  		},
  		colors: {
  			// 离线会记 v0.6.7: 注册 app-* 设计 token namespace, 让 bg-app-recording/text-app-transcript 等 utility 生效
  			'app-canvas': 'var(--app-canvas)',
  			'app-surface-1': 'var(--app-surface-1)',
  			'app-surface-2': 'var(--app-surface-2)',
  			'app-surface-3': 'var(--app-surface-3)',
  			'app-surface-4': 'var(--app-surface-4)',
  			'app-hairline': 'var(--app-hairline)',
  			'app-hairline-strong': 'var(--app-hairline-strong)',
  			'app-ink': 'var(--app-ink)',
  			'app-ink-muted': 'var(--app-ink-muted)',
  			'app-ink-subtle': 'var(--app-ink-subtle)',
  			'app-ink-tertiary': 'var(--app-ink-tertiary)',
  			'app-recording': 'var(--app-recording)',
  			'app-recording-soft': 'var(--app-recording-soft)',
  			'app-transcript': 'var(--app-transcript)',
  			'app-transcript-soft': 'var(--app-transcript-soft)',
  			'app-transcript-hover': 'var(--app-transcript-hover)',
  			'app-summary': 'var(--app-summary)',
  			'app-summary-soft': 'var(--app-summary-soft)',
  			'app-summary-deep': 'var(--app-summary-deep)',
  			'app-success': 'var(--app-success)',
  			'app-success-soft': 'var(--app-success-soft)',
  			'app-warning': 'var(--app-warning)',
  			'app-error': 'var(--app-error)',
  			'app-info': 'var(--app-info)',
  			background: 'hsl(var(--background))',
  			foreground: 'hsl(var(--foreground))',
  			border: 'hsl(var(--border))',
  			input: 'hsl(var(--input))',
  			ring: 'hsl(var(--ring))',
  			primary: {
  				DEFAULT: 'hsl(var(--primary))',
  				foreground: 'hsl(var(--primary-foreground))'
  			},
  			secondary: {
  				DEFAULT: 'hsl(var(--secondary))',
  				foreground: 'hsl(var(--secondary-foreground))'
  			},
  			tertiary: '#64748b',
  			card: {
  				DEFAULT: 'hsl(var(--card))',
  				foreground: 'hsl(var(--card-foreground))'
  			},
  			popover: {
  				DEFAULT: 'hsl(var(--popover))',
  				foreground: 'hsl(var(--popover-foreground))'
  			},
  			muted: {
  				DEFAULT: 'hsl(var(--muted))',
  				foreground: 'hsl(var(--muted-foreground))'
  			},
  			accent: {
  				DEFAULT: 'hsl(var(--accent))',
  				foreground: 'hsl(var(--accent-foreground))'
  			},
  			destructive: {
  				DEFAULT: 'hsl(var(--destructive))',
  				foreground: 'hsl(var(--destructive-foreground))'
  			},
  			chart: {
  				'1': 'hsl(var(--chart-1))',
  				'2': 'hsl(var(--chart-2))',
  				'3': 'hsl(var(--chart-3))',
  				'4': 'hsl(var(--chart-4))',
  				'5': 'hsl(var(--chart-5))'
  			}
  		},
  		borderRadius: {
  			// 保留 shadcn 默认 radius (legcy)
  			lg: 'var(--radius)',
  			md: 'calc(var(--radius) - 2px)',
  			sm: 'calc(var(--radius) - 4px)',
  			// 离线会记 namespace — 对齐 Linear/Raycast 几何
  			'app-xs': 'var(--app-radius-xs)',
  			'app-sm': 'var(--app-radius-sm)',
  			'app-md': 'var(--app-radius-md)',
  			'app-lg': 'var(--app-radius-lg)',
  			'app-xl': 'var(--app-radius-xl)',
  			'app-xxl': 'var(--app-radius-xxl)',
  			'app-pill': 'var(--app-radius-pill)',
  		},
  		boxShadow: {
  			'app-subtle': 'var(--app-shadow-subtle)',
  			'app-card': 'var(--app-shadow-card)',
  			'app-elevated': 'var(--app-shadow-elevated)',
  			'app-dialog': 'var(--app-shadow-dialog)',
  		},
  		keyframes: {
  			'accordion-down': {
  				from: {
  					height: '0'
  				},
  				to: {
  					height: 'var(--radix-accordion-content-height)'
  				}
  			},
  			'accordion-up': {
  				from: {
  					height: 'var(--radix-accordion-content-height)'
  				},
  				to: {
  					height: '0'
  				}
  			}
  		},
  		animation: {
  			'accordion-down': 'accordion-down 0.2s ease-out',
  			'accordion-up': 'accordion-up 0.2s ease-out'
  		}
  	}
  },
  plugins: [require("tailwindcss-animate")],
}