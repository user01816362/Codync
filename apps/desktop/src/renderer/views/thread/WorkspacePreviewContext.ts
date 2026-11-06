import { createContext } from 'react'

/** Opening a workspace path is always an explicit user action. */
export const WorkspacePreviewContext = createContext<((path: string) => void) | null>(null)
