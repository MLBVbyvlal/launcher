import { useState, useEffect, useRef } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { getLang, useT } from './i18n'

interface JvmPollResult {
  lines: string[]
  new_offset: number
  cleared: boolean
}

function lineClass(line: string): string {
  const lo = line.toLowerCase()
  if (/\[error\]|error\b/.test(lo)) return 'console-line-error'
  if (/\[warn\]|\bwarn\b|\bwarning\b/.test(lo)) return 'console-line-warn'
  if (/\[debug\]|\bdebug\b/.test(lo)) return 'console-line-debug'
  return 'console-line-info'
}

export default function ConsolePanel({ instanceName, running, onSelect }: {
  instanceName: string | null
  running: string[]
  onSelect: (name: string) => void
}) {
  const t = useT(getLang())
  const [lines, setLines] = useState<string[]>([])
  const [copied, setCopied] = useState(false)
  const endRef = useRef<HTMLDivElement>(null)
  const offsetRef = useRef(0)
  const linesRef = useRef<string[]>([])

  // Reset the view whenever the watched instance changes
  useEffect(() => {
    linesRef.current = []
    setLines([])
    offsetRef.current = 0
  }, [instanceName])

  // Stream JVM output via in-memory buffer (no file dependency)
  useEffect(() => {
    if (!instanceName) return
    const id = setInterval(async () => {
      try {
        const result = await invoke<JvmPollResult>('poll_jvm_output', {
          offset: offsetRef.current,
          instanceName,
        })
        if (result.cleared) {
          linesRef.current = []
          setLines([])
          offsetRef.current = 0
        } else if (result.lines.length > 0) {
          const next = [...linesRef.current.slice(-3000), ...result.lines]
          linesRef.current = next
          setLines([...next])
          offsetRef.current = result.new_offset
        }
      } catch { /* ignore */ }
    }, 200)
    return () => clearInterval(id)
  }, [instanceName])

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
    if (instanceName) invoke('open_instance_logs_folder', { instanceName }).catch(() => {})
  }

  const clearLog = () => {
    linesRef.current = []
    setLines([])
  }

  return (
    <div className="console-tab">
      <div className="console-toolbar">
        {(running.length > 0 || instanceName) ? (
          <select className="console-select" value={instanceName ?? ''} onChange={e => onSelect(e.target.value)}>
            {instanceName === null && <option value="" disabled>—</option>}
            {(instanceName && !running.includes(instanceName) ? [instanceName, ...running] : running).map(name => (
              <option key={name} value={name}>{name}</option>
            ))}
          </select>
        ) : (
          <span className="console-toolbar-title">{t('console.title')}</span>
        )}
        <span style={{ flex: 1 }} />
        <button className="btn-secondary" style={{ padding: '5px 12px', fontSize: 11 }} onClick={copyLog} disabled={!instanceName}>
          {copied ? `✓ ${t('console.copied')}` : t('console.copy')}
        </button>
        <button className="btn-secondary" style={{ padding: '5px 12px', fontSize: 11 }} onClick={openFolder} disabled={!instanceName}>
          {t('console.open_folder')}
        </button>
        <span className="console-line-count">{lines.length} {t('console.lines')}</span>
        <button className="btn-secondary" style={{ padding: '5px 12px', fontSize: 11 }} onClick={clearLog} disabled={!instanceName}>
          {t('console.clear')}
        </button>
      </div>

      <div className="console-log console-log-tab">
        {!instanceName ? (
          <div className="console-empty">{t('console.no_running')}</div>
        ) : lines.length === 0 ? (
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
