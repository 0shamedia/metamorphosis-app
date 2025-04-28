declare module '@tauri-apps/api/fs' {
  export interface FsApi {
    resolveResource: (path: string) => Promise<string>;
    exists: (path: string) => Promise<boolean>;
    createDir: (path: string, options?: { recursive: boolean }) => Promise<void>;
    writeBinaryFile: (path: string, contents: Uint8Array) => Promise<void>;
    // Add other fs functions if needed
  }
  const fs: FsApi;
  export = fs;
}