import { useEffect, type ReactNode } from 'react'
import { color } from '../lib/tokens'

export function Overlay({
  onClose,
  children,
  top = 112,
  width = 720,
  label,
}: {
  onClose: () => void
  children: ReactNode
  top?: number
  width?: number
  label: string
}) {
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose()
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [onClose])

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-label={label}
      onClick={onClose}
      style={{
        position: 'fixed',
        inset: 0,
        background: 'rgba(21,33,40,0.44)',
        zIndex: 50,
        display: 'flex',
        justifyContent: 'center',
        alignItems: 'flex-start',
        paddingTop: top,
      }}
    >
      <div
        onClick={(e) => e.stopPropagation()}
        style={{
          width,
          maxWidth: 'calc(100vw - 48px)',
          maxHeight: 'calc(100vh - 160px)',
          display: 'flex',
          flexDirection: 'column',
          background: color.surface,
          borderRadius: 16,
          boxShadow: '0 28px 70px rgba(15,25,32,0.34), 0 2px 8px rgba(15,25,32,0.16)',
          overflow: 'hidden',
        }}
      >
        {children}
      </div>
    </div>
  )
}
