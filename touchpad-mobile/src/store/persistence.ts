import { defineStore } from "pinia";
import { Store } from "@tauri-apps/plugin-store";
import { TuneSetting } from "@/ipc/types";

interface StoreValue {
  tune: TuneSetting;
  [key: string]: unknown;
}

const defaultStore: StoreValue = {
  tune: {
    sensitivity: 1.0,
    invertY: false,
    invertX: false,
  },
};

const store = await Store.load("store.bin", {
  defaults: defaultStore,
  autoSave: true,
});

let initPromise: Promise<void> | null = null;

export const usePersistenceStore = defineStore("persistence", {
  state: () => ({
    tune: defaultStore.tune,
    __initialized: false,
  }),
  actions: {
    async get<K extends keyof StoreValue>(key: K): Promise<StoreValue[K]> {
      try {
        let value = await store.get(key as string);
        if (key === "tune") {
          this.tune = value as TuneSetting;
        }
        return value;
      } catch (error) {
        console.error(`Error getting ${key}:`, error);
        return undefined;
      }
    },

    async set<K extends keyof StoreValue>(
      key: K,
      value: StoreValue[K],
      immediate = false,
    ) {
      try {
        await store.set(key as string, value);
      } catch (error) {
        console.error(`Error setting ${key}:`, error);
      }
      if (immediate) {
        await store.save();
      }
      if (key === "tune") {
        this.tune = value as TuneSetting;
      }
    },

    async _init() {
      await this.get("tune");
      this.__initialized = true;
    },

    // 确保初始化完成
    async ensureInitialized() {
      if (!this.__initialized) {
        if (!initPromise) {
          initPromise = this._init();
        }
        await initPromise;
      }
    },
  },
});
