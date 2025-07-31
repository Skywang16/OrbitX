/**
 * 快捷键状态管理 Store
 * 
 * 使用 Pinia 管理快捷键配置的响应式状态和操作方法
 */

import { defineStore } from 'pinia';
import { ref, computed, watch } from 'vue';
import { ShortcutApi } from '@/api/shortcuts';
import type {
  ShortcutsConfig,
  ShortcutBinding,
  ShortcutCategory,
  Platform,
  ShortcutValidationResult,
  ConflictDetectionResult,
  ShortcutStatistics,
  ShortcutSearchOptions,
  ShortcutSearchResult,
  ShortcutOperationOptions,
} from '@/api/shortcuts/types';

/**
 * 快捷键 Store 状态接口
 */
interface ShortcutStoreState {
  /** 快捷键配置 */
  config: ShortcutsConfig | null;
  /** 当前平台 */
  currentPlatform: Platform | null;
  /** 统计信息 */
  statistics: ShortcutStatistics | null;
  /** 最后一次验证结果 */
  lastValidation: ShortcutValidationResult | null;
  /** 最后一次冲突检测结果 */
  lastConflictDetection: ConflictDetectionResult | null;
  /** 加载状态 */
  loading: boolean;
  /** 错误信息 */
  error: string | null;
  /** 是否已初始化 */
  initialized: boolean;
}

export const useShortcutStore = defineStore('shortcuts', () => {
  // 状态
  const state = ref<ShortcutStoreState>({
    config: null,
    currentPlatform: null,
    statistics: null,
    lastValidation: null,
    lastConflictDetection: null,
    loading: false,
    error: null,
    initialized: false,
  });

  // 计算属性
  const hasConfig = computed(() => state.value.config !== null);
  
  const hasConflicts = computed(() => 
    state.value.lastConflictDetection?.has_conflicts ?? false
  );
  
  const hasValidationErrors = computed(() => 
    state.value.lastValidation && !state.value.lastValidation.is_valid
  );
  
  const totalShortcuts = computed(() => 
    state.value.statistics?.total_count ?? 0
  );

  const shortcutsByCategory = computed(() => {
    if (!state.value.config) return null;
    
    return {
      global: state.value.config.global,
      terminal: state.value.config.terminal,
      custom: state.value.config.custom,
    };
  });

  // 操作方法
  
  /**
   * 初始化快捷键 Store
   */
  const initialize = async (): Promise<void> => {
    if (state.value.initialized) return;
    
    state.value.loading = true;
    state.value.error = null;
    
    try {
      // 并行加载配置、平台信息和统计信息
      const [config, platform, statistics] = await Promise.all([
        ShortcutApi.getConfig(),
        ShortcutApi.getCurrentPlatform(),
        ShortcutApi.getStatistics(),
      ]);
      
      state.value.config = config;
      state.value.currentPlatform = platform;
      state.value.statistics = statistics;
      state.value.initialized = true;
      
      // 初始化时进行一次验证和冲突检测
      await Promise.all([
        validateCurrentConfig(),
        detectCurrentConflicts(),
      ]);
    } catch (error) {
      state.value.error = `初始化快捷键配置失败: ${error}`;
      throw error;
    } finally {
      state.value.loading = false;
    }
  };

  /**
   * 刷新配置
   */
  const refreshConfig = async (): Promise<void> => {
    state.value.loading = true;
    state.value.error = null;
    
    try {
      const [config, statistics] = await Promise.all([
        ShortcutApi.getConfig(),
        ShortcutApi.getStatistics(),
      ]);
      
      state.value.config = config;
      state.value.statistics = statistics;
      
      // 刷新后重新验证
      await Promise.all([
        validateCurrentConfig(),
        detectCurrentConflicts(),
      ]);
    } catch (error) {
      state.value.error = `刷新快捷键配置失败: ${error}`;
      throw error;
    } finally {
      state.value.loading = false;
    }
  };

  /**
   * 更新配置
   */
  const updateConfig = async (
    config: ShortcutsConfig,
    options: ShortcutOperationOptions = {}
  ): Promise<void> => {
    state.value.loading = true;
    state.value.error = null;
    
    try {
      await ShortcutApi.updateConfig(config, options);
      state.value.config = config;
      
      // 更新统计信息
      state.value.statistics = await ShortcutApi.getStatistics();
      
      // 重新验证和检测冲突
      await Promise.all([
        validateCurrentConfig(),
        detectCurrentConflicts(),
      ]);
    } catch (error) {
      state.value.error = `更新快捷键配置失败: ${error}`;
      throw error;
    } finally {
      state.value.loading = false;
    }
  };

  /**
   * 验证当前配置
   */
  const validateCurrentConfig = async (): Promise<ShortcutValidationResult> => {
    if (!state.value.config) {
      throw new Error('没有可验证的配置');
    }
    
    try {
      const result = await ShortcutApi.validateConfig(state.value.config);
      state.value.lastValidation = result;
      return result;
    } catch (error) {
      state.value.error = `验证快捷键配置失败: ${error}`;
      throw error;
    }
  };

  /**
   * 检测当前配置的冲突
   */
  const detectCurrentConflicts = async (): Promise<ConflictDetectionResult> => {
    if (!state.value.config) {
      throw new Error('没有可检测的配置');
    }
    
    try {
      const result = await ShortcutApi.detectConflicts(state.value.config);
      state.value.lastConflictDetection = result;
      return result;
    } catch (error) {
      state.value.error = `检测快捷键冲突失败: ${error}`;
      throw error;
    }
  };

  /**
   * 添加快捷键
   */
  const addShortcut = async (
    category: ShortcutCategory,
    shortcut: ShortcutBinding,
    options: ShortcutOperationOptions = {}
  ): Promise<void> => {
    state.value.loading = true;
    state.value.error = null;
    
    try {
      await ShortcutApi.addShortcut(category, shortcut, options);
      await refreshConfig();
    } catch (error) {
      state.value.error = `添加快捷键失败: ${error}`;
      throw error;
    } finally {
      state.value.loading = false;
    }
  };

  /**
   * 删除快捷键
   */
  const removeShortcut = async (
    category: ShortcutCategory,
    index: number
  ): Promise<ShortcutBinding> => {
    state.value.loading = true;
    state.value.error = null;
    
    try {
      const removedShortcut = await ShortcutApi.removeShortcut(category, index);
      await refreshConfig();
      return removedShortcut;
    } catch (error) {
      state.value.error = `删除快捷键失败: ${error}`;
      throw error;
    } finally {
      state.value.loading = false;
    }
  };

  /**
   * 更新快捷键
   */
  const updateShortcut = async (
    category: ShortcutCategory,
    index: number,
    shortcut: ShortcutBinding,
    options: ShortcutOperationOptions = {}
  ): Promise<void> => {
    state.value.loading = true;
    state.value.error = null;
    
    try {
      await ShortcutApi.updateShortcut(category, index, shortcut, options);
      await refreshConfig();
    } catch (error) {
      state.value.error = `更新快捷键失败: ${error}`;
      throw error;
    } finally {
      state.value.loading = false;
    }
  };

  /**
   * 搜索快捷键
   */
  const searchShortcuts = async (
    options: ShortcutSearchOptions
  ): Promise<ShortcutSearchResult> => {
    try {
      return await ShortcutApi.searchShortcuts(options);
    } catch (error) {
      state.value.error = `搜索快捷键失败: ${error}`;
      throw error;
    }
  };

  /**
   * 重置到默认配置
   */
  const resetToDefaults = async (): Promise<void> => {
    state.value.loading = true;
    state.value.error = null;
    
    try {
      await ShortcutApi.resetToDefaults();
      await refreshConfig();
    } catch (error) {
      state.value.error = `重置快捷键配置失败: ${error}`;
      throw error;
    } finally {
      state.value.loading = false;
    }
  };

  /**
   * 导出配置
   */
  const exportConfig = async (): Promise<string> => {
    try {
      return await ShortcutApi.exportConfig();
    } catch (error) {
      state.value.error = `导出快捷键配置失败: ${error}`;
      throw error;
    }
  };

  /**
   * 导入配置
   */
  const importConfig = async (json: string): Promise<void> => {
    state.value.loading = true;
    state.value.error = null;
    
    try {
      await ShortcutApi.importConfig(json);
      await refreshConfig();
    } catch (error) {
      state.value.error = `导入快捷键配置失败: ${error}`;
      throw error;
    } finally {
      state.value.loading = false;
    }
  };

  /**
   * 清除错误
   */
  const clearError = (): void => {
    state.value.error = null;
  };

  // 监听配置变化，自动重新验证
  watch(
    () => state.value.config,
    async (newConfig) => {
      if (newConfig && state.value.initialized) {
        try {
          await Promise.all([
            validateCurrentConfig(),
            detectCurrentConflicts(),
          ]);
        } catch (error) {
          console.warn('自动验证失败:', error);
        }
      }
    },
    { deep: true }
  );

  return {
    // 状态
    state: state.value,
    
    // 计算属性
    hasConfig,
    hasConflicts,
    hasValidationErrors,
    totalShortcuts,
    shortcutsByCategory,
    
    // 操作方法
    initialize,
    refreshConfig,
    updateConfig,
    validateCurrentConfig,
    detectCurrentConflicts,
    addShortcut,
    removeShortcut,
    updateShortcut,
    searchShortcuts,
    resetToDefaults,
    exportConfig,
    importConfig,
    clearError,
  };
});
