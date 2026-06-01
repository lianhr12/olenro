import {
  useMutation,
  useQuery,
  useQueryClient,
  keepPreviousData,
} from "@tanstack/react-query";
import {
  skillsApi,
  type SkillBackupEntry,
  type DiscoverableSkill,
  type ImportSkillSelection,
  type InstalledSkill,
  type SkillUpdateInfo,
  type SkillsShSearchResult,
} from "@/lib/api/skills";
import type { AppId } from "@/lib/api/types";
import { mergeImportedSkills } from "@/hooks/useSkills.helpers";

/**
 * 查询所有已安装的 Skills
 * 使用 staleTime: Infinity 和 placeholderData: keepPreviousData
 * 实现首次进入使用缓存，只有刷新时才重新获取
 */
export function useInstalledSkills() {
  return useQuery({
    queryKey: ["skills", "installed"],
    queryFn: () => skillsApi.getInstalled(),
    staleTime: Infinity,
    placeholderData: keepPreviousData,
  });
}

export function useSkillBackups() {
  return useQuery({
    queryKey: ["skills", "backups"],
    queryFn: () => skillsApi.getBackups(),
    enabled: false,
  });
}

export function useDeleteSkillBackup() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (backupId: string) => skillsApi.deleteBackup(backupId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["skills", "backups"] });
    },
  });
}

/**
 * 发现可安装的 Skills（从仓库获取）
 * 使用 staleTime: Infinity 和 placeholderData: keepPreviousData
 * 实现首次进入使用缓存，只有刷新时才重新获取
 */
export function useDiscoverableSkills() {
  return useQuery({
    queryKey: ["skills", "discoverable"],
    queryFn: () => skillsApi.discoverAvailable(),
    staleTime: Infinity,
    placeholderData: keepPreviousData,
  });
}

/**
 * 安装 Skill
 * 成功后直接更新缓存，不触发重新加载/刷新
 */
export function useInstallSkill() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({
      skill,
      currentApp,
    }: {
      skill: DiscoverableSkill;
      currentApp: AppId;
    }) => skillsApi.installUnified(skill, currentApp),
    onSuccess: (installedSkill, _vars, _ctx) => {
      const { skill } = _vars;
      // 直接更新 installed 缓存
      queryClient.setQueryData<InstalledSkill[]>(
        ["skills", "installed"],
        (oldData) => {
          if (!oldData) return [installedSkill];
          return [...oldData, installedSkill];
        },
      );

      // 更新 discoverable 缓存中对应技能的 installed 状态
      const installName =
        skill.directory.split(/[/\\]/).pop()?.toLowerCase() ||
        skill.directory.toLowerCase();
      const skillKey = `${installName}:${skill.repoOwner.toLowerCase()}:${skill.repoName.toLowerCase()}`;

      queryClient.setQueryData<DiscoverableSkill[]>(
        ["skills", "discoverable"],
        (oldData) => {
          if (!oldData) return oldData;
          return oldData.map((s) => {
            if (s.key === skillKey) {
              return { ...s, installed: true };
            }
            return s;
          });
        },
      );
    },
  });
}

/**
 * 卸载 Skill
 * 成功后直接更新缓存，不触发重新加载/刷新
 */
export function useUninstallSkill() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, skillKey }: { id: string; skillKey: string }) =>
      skillsApi
        .uninstallUnified(id)
        .then((result) => ({ ...result, skillKey })),
    onSuccess: ({ skillKey }, _vars) => {
      // 直接更新 installed 缓存，移除该 skill
      queryClient.setQueryData<InstalledSkill[]>(
        ["skills", "installed"],
        (oldData) => {
          if (!oldData) return oldData;
          return oldData.filter((s) => s.id !== _vars.id);
        },
      );

      // 更新 discoverable 缓存中对应技能的 installed 状态
      queryClient.setQueryData<DiscoverableSkill[]>(
        ["skills", "discoverable"],
        (oldData) => {
          if (!oldData) return oldData;
          return oldData.map((s) => {
            if (s.key === skillKey) {
              return { ...s, installed: false };
            }
            return s;
          });
        },
      );
    },
  });
}

export function useRestoreSkillBackup() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({
      backupId,
      currentApp,
    }: {
      backupId: string;
      currentApp: AppId;
    }) => skillsApi.restoreBackup(backupId, currentApp),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["skills", "installed"] });
      queryClient.invalidateQueries({ queryKey: ["skills", "backups"] });
    },
  });
}

/**
 * 切换 Skill 在特定应用的启用状态
 */
export function useToggleSkillApp() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({
      id,
      app,
      enabled,
    }: {
      id: string;
      app: AppId;
      enabled: boolean;
    }) => skillsApi.toggleApp(id, app, enabled),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["skills", "installed"] });
    },
  });
}

/**
 * 批量切换多个 Skill/应用的启用状态（顺序执行，避免并发写同一目录）
 */
export function useToggleSkillAppsBulk() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async ({
      targets,
      enabled,
    }: {
      targets: Array<{ id: string; app: AppId }>;
      enabled: boolean;
    }) => {
      for (const { id, app } of targets) {
        await skillsApi.toggleApp(id, app, enabled);
      }
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["skills", "installed"] });
    },
  });
}

/**
 * 扫描未管理的 Skills
 */
export function useScanUnmanagedSkills() {
  return useQuery({
    queryKey: ["skills", "unmanaged"],
    queryFn: () => skillsApi.scanUnmanaged(),
    enabled: false, // 手动触发
  });
}

/**
 * 从应用目录导入 Skills
 * 成功后直接更新缓存，不触发重新加载/刷新
 */
export function useImportSkillsFromApps() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (imports: ImportSkillSelection[]) =>
      skillsApi.importFromApps(imports),
    onSuccess: (importedSkills) => {
      // 直接更新 installed 缓存
      queryClient.setQueryData<InstalledSkill[]>(
        ["skills", "installed"],
        (oldData) => mergeImportedSkills(oldData, importedSkills),
      );
      // 刷新 unmanaged 列表（已被导入的应该移除）
      queryClient.invalidateQueries({ queryKey: ["skills", "unmanaged"] });
    },
  });
}

/**
 * 获取仓库列表
 */
export function useSkillRepos() {
  return useQuery({
    queryKey: ["skills", "repos"],
    queryFn: () => skillsApi.getRepos(),
  });
}

/**
 * 添加仓库
 */
export function useAddSkillRepo() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: skillsApi.addRepo,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["skills", "repos"] });
      queryClient.invalidateQueries({ queryKey: ["skills", "discoverable"] });
    },
  });
}

/**
 * 删除仓库
 */
export function useRemoveSkillRepo() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ owner, name }: { owner: string; name: string }) =>
      skillsApi.removeRepo(owner, name),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["skills", "repos"] });
      queryClient.invalidateQueries({ queryKey: ["skills", "discoverable"] });
    },
  });
}

/**
 * 从 ZIP 文件安装 Skills
 * 成功后直接更新缓存，不触发重新加载/刷新
 */
export function useInstallSkillsFromZip() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({
      filePath,
      currentApp,
    }: {
      filePath: string;
      currentApp: AppId;
    }) => skillsApi.installFromZip(filePath, currentApp),
    onSuccess: (installedSkills) => {
      // 直接更新 installed 缓存
      queryClient.setQueryData<InstalledSkill[]>(
        ["skills", "installed"],
        (oldData) => {
          if (!oldData) return installedSkills;
          return [...oldData, ...installedSkills];
        },
      );
    },
  });
}

// ========== 更新检测 ==========

/**
 * 检查 Skills 更新（手动触发）
 */
export function useCheckSkillUpdates() {
  return useQuery({
    queryKey: ["skills", "updates"],
    queryFn: () => skillsApi.checkUpdates(),
    enabled: false,
    staleTime: 5 * 60 * 1000,
  });
}

/**
 * 更新单个 Skill
 */
export function useUpdateSkill() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => skillsApi.updateSkill(id),
    onSuccess: (updatedSkill) => {
      queryClient.setQueryData<InstalledSkill[]>(
        ["skills", "installed"],
        (oldData) => {
          if (!oldData) return [updatedSkill];
          return oldData.map((s) =>
            s.id === updatedSkill.id ? updatedSkill : s,
          );
        },
      );
      queryClient.setQueryData<SkillUpdateInfo[]>(
        ["skills", "updates"],
        (oldData) => {
          if (!oldData) return oldData;
          return oldData.filter((u) => u.id !== updatedSkill.id);
        },
      );
    },
  });
}

// ========== skills.sh 搜索 ==========

/**
 * 搜索 skills.sh 公共目录
 * 使用 300ms staleTime 和 keepPreviousData 实现平滑搜索体验
 */
export function useSearchSkillsSh(
  query: string,
  limit: number,
  offset: number,
) {
  return useQuery({
    queryKey: ["skills", "skillssh", query, limit, offset],
    queryFn: () => skillsApi.searchSkillsSh(query, limit, offset),
    enabled: query.length >= 2,
    staleTime: 5 * 60 * 1000,
    placeholderData: keepPreviousData,
  });
}

/**
 * 获取 skills.sh 热门技能（按安装量倒序）
 * 用于用户尚未输入搜索词时默认展示，帮助新手快速发现技能
 */
export function usePopularSkillsSh(limit: number, enabled: boolean) {
  return useQuery({
    queryKey: ["skills", "skillssh", "popular", limit],
    queryFn: () => skillsApi.getPopularSkillsSh(limit),
    enabled,
    staleTime: 30 * 60 * 1000,
  });
}

// ========== 辅助类型 ==========

export type {
  InstalledSkill,
  DiscoverableSkill,
  ImportSkillSelection,
  SkillBackupEntry,
  SkillUpdateInfo,
  SkillsShSearchResult,
  AppId,
};

// Re-export skill detail types
export type { SkillDetail, SkillFileInfo } from "@/lib/api/skills";

// ========== Skill 详情查看 ==========

/**
 * 获取 Skill 完整详情（包含 manifest 内容和文件列表）
 */
export function useSkillDetail(skillId: string | null) {
  return useQuery({
    queryKey: ["skills", "detail", skillId],
    queryFn: () => skillsApi.getSkillDetail(skillId!),
    enabled: !!skillId,
    staleTime: 30 * 1000, // 30 秒缓存
  });
}

/**
 * 读取 Skill 文件内容
 */
export function useSkillFileContent(
  skillId: string | null,
  filePath: string | null,
) {
  return useQuery({
    queryKey: ["skills", "file", skillId, filePath],
    queryFn: () => skillsApi.readSkillFile(skillId!, filePath!),
    enabled: !!skillId && !!filePath,
    staleTime: 10 * 1000, // 10 秒缓存
  });
}

/**
 * 写入 Skill 文件内容
 */
export function useWriteSkillFile() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({
      skillId,
      filePath,
      content,
    }: {
      skillId: string;
      filePath: string;
      content: string;
    }) => skillsApi.writeSkillFile(skillId, filePath, content),
    onSuccess: (_result, { skillId }) => {
      // 清除文件内容缓存
      queryClient.invalidateQueries({
        queryKey: ["skills", "file", skillId],
      });
      // 清除详情缓存
      queryClient.invalidateQueries({
        queryKey: ["skills", "detail", skillId],
      });
    },
  });
}
