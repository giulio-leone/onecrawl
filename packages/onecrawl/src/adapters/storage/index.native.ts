/**
 * Storage Adapters - React Native Compatible
 * Excludes FsStorageAdapter (requires Node.js fs/path/os)
 */

export {
  MemoryStorageAdapter,
  createMemoryStorageAdapter,
} from "./memory-storage.adapter.js";

export {
  AsyncStorageAdapter,
  createAsyncStorageAdapter,
  type AsyncStorageStatic,
} from "./async-storage.adapter.js";

export {
  MmkvStorageAdapter,
  createMmkvStorageAdapter,
  type MmkvInstance,
} from "./mmkv-storage.adapter.js";
