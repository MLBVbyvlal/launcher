import { useState, useEffect, useRef } from 'react'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { invoke } from '@tauri-apps/api/core'
import { getLang, useT } from './i18n'
import './App.css'

interface ConsoleInfo {
  instance_name: string
  log_path: string
}

interface PollResult {
  lines: string[]
  new_offset: number
}

function lineClass(line: string): string {
  const lo = line.toLowerCase()
  if (/\[error\]|error\b/.test(lo)) return 'console-line-error'
  if (/\[warn\]|\bwarn\b|\bwarning\b/.test(lo)) return 'console-line-warn'
  if (/\[debug\]|\bdebug\b/.test(lo)) return 'console-line-debug'
  return 'console-line-info'
}

export default function ConsoleWindow() {
  const t = useT(getLang())
  const [info, setInfo] = useState<ConsoleInfo | null>(null)
  const [lines, setLines] = useState<string[]>([])
  const [copied, setCopied] = useState(false)
  const endRef = useRef<HTMLDivElement>(null)
  const offsetRef = useRef(0)
  const linesRef = useRef<string[]>([])

  // Get console info from Rust (instance name + log path)
  useEffect(() => {
    const load = () => {
      invoke<ConsoleInfo | null>('get_console_info')
        .then(i => { if (i) setInfo(i) })
        .catch(() => {})
    }
    load()
    // Retry a few times in case the window opened before the state was set
    const id = setInterval(() => {
      if (info) { clearInterval(id); return }
      load()
    }, 400)
    return () => clearInterval(id)
  }, []) // eslint-disable-line react-hooks/exhaustive-deps

  // Poll log file for new lines
  useEffect(() => {
    if (!info) return
    const id = setInterval(async () => {
      try {
        const result = await invoke<PollResult>('poll_console', {
          logPath: info.log_path,
          offset: offsetRef.current,
        })
        if (result.lines.length > 0) {
          const next = [...linesRef.current.slice(-3000), ...result.lines]
          linesRef.current = next
          setLines([...next])
          offsetRef.current = result.new_offset
        }
      } catch { /* ignore */ }
    }, 200)
    return () => clearInterval(id)
  }, [info])

  // Auto-scroll to bottom when new lines arrive
  useEffect(() => {
    endRef.current?.scrollIntoView({ behavior: 'instant' })
  }, [lines])

  const copyLog = () => {
    navigator.clipboard.writeText(linesRef.current.join('\n')).then(() => {
      setCopied(true)
      setTimeout(() => setCopied(false), 1600)
    })
  }

  const openFolder = () => {
    if (info) invoke('open_instance_logs_folder', { instanceName: info.instance_name }).catch(() => {})
  }

  const clearLog = () => {
    linesRef.current = []
    setLines([])
  }

  return (
    <div className="console-root">
      <div className="bg-canvas" style={{ pointerEvents: 'none', position: 'fixed' }}>
        <div className="bg-grid" />
        <div className="orb orb-1" />
        <div className="orb orb-2" />
      </div>

      {/* Titlebar */}
      <div
        className="console-titlebar"
        onMouseDown={e => {
          if (!(e.target as HTMLElement).closest('button')) {
            getCurrentWindow().startDragging().catch(() => {})
          }
        }}
      >
        <div className="console-titlebar-icon">
          <svg viewBox="0 0 16 16" fill="none" width="14" height="14">
            <rect x="1" y="3" width="14" height="10" rx="2" stroke="currentColor" strokeWidth="1.2"/>
            <path d="M4 8l2.5 2.5L4 13" stroke="currentColor" strokeWidth="1.3" strokeLinecap="round" strokeLinejoin="round"/>
            <path d="M9.5 13h2.5" stroke="currentColor" strokeWidth="1.3" strokeLinecap="round"/>
          </svg>
        </div>
        <span className="console-titlebar-title">
          {t('console.title')}{info ? ` — ${info.instance_name}` : ''}
        </span>
        <button className="wc cls" style={{ marginLeft: 'auto' }}
          onClick={() => getCurrentWindow().close().catch(() => {})}>✕</button>
      </div>

      {/* Toolbar */}
      <div className="console-toolbar">
        <button className="btn-secondary" style={{ padding: '5px 12px', fontSize: 11 }} onClick={copyLog}>
          {copied ? '✓ Copied' : t('console.copy')}
        </button>
        <button className="btn-secondary" style={{ padding: '5px 12px', fontSize: 11 }} onClick={openFolder}>
          {t('console.open_folder')}
        </button>
        <span className="console-line-count">{lines.length} {t('console.lines')}</span>
        <button className="btn-secondary" style={{ padding: '5px 12px', fontSize: 11 }} onClick={clearLog}>
          {t('console.clear')}
        </button>
      </div>

      {/* Log area */}
      <div className="console-log">
        {lines.length === 0 ? (
          <div className="console-empty">{t('console.empty')}</div>
        ) : (
          lines.map((line, i) => (
            <div key={i} className={`console-line ${lineClass(line)}`}>{line}</div>
          ))
        )}
        <div ref={endRef} />
      </div>
    </div>
  )
}
