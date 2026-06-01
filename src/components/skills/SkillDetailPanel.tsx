import React, { useState, useEffect, useMemo, useCallback } from "react";
import {
  FolderOpen,
  FileText,
  ChevronRight,
  ChevronDown,
  ExternalLink,
  Loader2,
  Pencil,
  Save,
  X,
} from "lucide-react";
import { useTranslation } from "react-i18next";
import { FullScreenPanel } from "@/components/common/FullScreenPanel";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import MarkdownEditor from "@/components/MarkdownEditor";
import { skillsApi, type SkillFileInfo } from "@/lib/api/skills";
import { useSkillDetail, useWriteSkillFile } from "@/hooks/useSkills";
import { toast } from "sonner";
import { SKILLS_APP_IDS } from "@/config/appConfig";

interface SkillDetailPanelProps {
  skillId: string | null;
  isOpen: boolean;
  onClose: () => void;
}

type ViewMode = "overview" | "files" | "file-content";

interface FileTreeNode {
  path: string;
  name: string;
  isDir: boolean;
  children: FileTreeNode[];
  file: SkillFileInfo;
}

function buildFileTree(files: SkillFileInfo[]): FileTreeNode[] {
  const root: FileTreeNode[] = [];
  const pathMap = new Map<string, FileTreeNode>();

  // Sort files by path
  const sorted = [...files].sort((a, b) => a.path.localeCompare(b.path));

  for (const file of sorted) {
    const parts = file.path.split("/");
    let currentLevel = root;
    let currentPath = "";

    for (let i = 0; i < parts.length; i++) {
      const part = parts[i];
      const isLast = i === parts.length - 1;
      const nodePath = currentPath ? `${currentPath}/${part}` : part;

      if (isLast) {
        const node: FileTreeNode = {
          path: nodePath,
          name: part,
          isDir: file.isDir,
          children: [],
          file,
        };
        pathMap.set(nodePath, node);
        currentLevel.push(node);
      } else {
        if (!pathMap.has(nodePath)) {
          const dirNode: FileTreeNode = {
            path: nodePath,
            name: part,
            isDir: true,
            children: [],
            file: {
              path: nodePath,
              name: part,
              sizeBytes: 0,
              isDir: true,
              modifiedAt: 0,
            },
          };
          pathMap.set(nodePath, dirNode);
          currentLevel.push(dirNode);
        }
        currentLevel = pathMap.get(nodePath)!.children;
        currentPath = nodePath;
      }
    }
  }

  return root;
}

export const SkillDetailPanel: React.FC<SkillDetailPanelProps> = ({
  skillId,
  isOpen,
  onClose,
}) => {
  const { t } = useTranslation();
  const [viewMode, setViewMode] = useState<ViewMode>("overview");
  const [selectedFile, setSelectedFile] = useState<SkillFileInfo | null>(null);
  const [fileContent, setFileContent] = useState<string | null>(null);
  const [editedContent, setEditedContent] = useState<string | null>(null);
  const [isLoadingFile, setIsLoadingFile] = useState(false);
  const [expandedDirs, setExpandedDirs] = useState<Set<string>>(new Set());
  const [isEditing, setIsEditing] = useState(false);
  const [manifestViewMode, setManifestViewMode] = useState<"rendered" | "source">("rendered");

  const { data: skillDetail, isLoading, refetch } = useSkillDetail(skillId);
  const writeFileMutation = useWriteSkillFile();

  // Reset state when panel opens with different skill
  useEffect(() => {
    if (isOpen && skillId) {
      setViewMode("overview");
      setSelectedFile(null);
      setFileContent(null);
      setEditedContent(null);
      setExpandedDirs(new Set());
      setIsEditing(false);
      setManifestViewMode("rendered");
    }
  }, [isOpen, skillId]);

  const handleOpenInExplorer = async () => {
    if (!skillId) return;
    try {
      await skillsApi.openSkillDirectory(skillId);
    } catch (error) {
      toast.error(t("common.error"), { description: String(error) });
    }
  };

  const handleFileSelect = async (file: SkillFileInfo) => {
    if (file.isDir) {
      // Toggle directory expansion
      setExpandedDirs((prev) => {
        const next = new Set(prev);
        if (next.has(file.path)) {
          next.delete(file.path);
        } else {
          next.add(file.path);
        }
        return next;
      });
    } else {
      // Load file content
      if (!skillId) return;
      setSelectedFile(file);
      setViewMode("file-content");
      setIsLoadingFile(true);
      setIsEditing(false);
      try {
        const content = await skillsApi.readSkillFile(skillId, file.path);
        setFileContent(content);
        setEditedContent(content);
      } catch (error) {
        toast.error(t("common.error"), { description: String(error) });
        setFileContent(null);
        setEditedContent(null);
      } finally {
        setIsLoadingFile(false);
      }
    }
  };

  const handleSaveFile = async () => {
    if (!skillId || !selectedFile || editedContent === null) return;

    try {
      await writeFileMutation.mutateAsync({
        skillId,
        filePath: selectedFile.path,
        content: editedContent,
      });
      toast.success(t("skills.detail.saveSuccess"));
      setFileContent(editedContent);
      setIsEditing(false);
      // Refresh detail to update file list
      refetch();
    } catch (error) {
      toast.error(t("skills.detail.saveFailed"), { description: String(error) });
    }
  };

  const handleCancelEdit = () => {
    setEditedContent(fileContent);
    setIsEditing(false);
  };

  const fileTree = useMemo(
    () => (skillDetail ? buildFileTree(skillDetail.files) : []),
    [skillDetail]
  );

  const renderFileTree = useCallback(
    (nodes: FileTreeNode[], depth = 0) => {
      return nodes.map((node) => {
        const isExpanded = expandedDirs.has(node.path);
        const hasChildren = node.children.length > 0;

        return (
          <div key={node.path}>
            <div
              className="flex items-center gap-1.5 px-2 py-1.5 hover:bg-muted/50 cursor-pointer rounded-md"
              style={{ paddingLeft: `${depth * 16 + 8}px` }}
              onClick={() => handleFileSelect(node.file)}
            >
              {node.isDir ? (
                isExpanded ? (
                  <ChevronDown size={14} />
                ) : (
                  <ChevronRight size={14} />
                )
              ) : (
                <span className="w-3.5" />
              )}
              {node.isDir ? (
                <FolderOpen size={14} className="text-amber-500" />
              ) : (
                <FileText size={14} className="text-muted-foreground" />
              )}
              <span className="text-sm truncate">{node.name}</span>
              {!node.isDir && (
                <span className="text-xs text-muted-foreground ml-auto">
                  {formatFileSize(node.file.sizeBytes)}
                </span>
              )}
            </div>
            {node.isDir && isExpanded && hasChildren && (
              <div>{renderFileTree(node.children, depth + 1)}</div>
            )}
          </div>
        );
      });
    },
    [expandedDirs]
  );

  const formatFileSize = (bytes: number): string => {
    if (bytes === 0) return "0 B";
    const k = 1024;
    const sizes = ["B", "KB", "MB"];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return `${parseFloat((bytes / Math.pow(k, i)).toFixed(1))} ${sizes[i]}`;
  };

  const formatDate = (timestamp: number): string => {
    return new Date(timestamp * 1000).toLocaleString();
  };

  const isMarkdownFile = (filename: string): boolean => {
    return /\.(md|markdown|mdx)$/i.test(filename);
  };

  const enabledApps = useMemo(() => {
    if (!skillDetail) return [];
    return SKILLS_APP_IDS.filter(
      (app) => skillDetail.apps[app as keyof typeof skillDetail.apps]
    );
  }, [skillDetail]);

  return (
    <FullScreenPanel
      isOpen={isOpen}
      title={skillDetail?.name || t("skills.detail.title")}
      onClose={onClose}
      footer={
        <>
          <Button variant="outline" onClick={onClose}>
            {t("common.close")}
          </Button>
          <Button variant="outline" onClick={handleOpenInExplorer}>
            <FolderOpen size={14} className="mr-1.5" />
            {t("skills.detail.openFolder")}
          </Button>
        </>
      }
    >
      {isLoading ? (
        <div className="flex items-center justify-center py-12">
          <Loader2 size={24} className="animate-spin mr-2" />
          <span>{t("common.loading")}</span>
        </div>
      ) : !skillDetail ? (
        <div className="text-center py-12 text-muted-foreground">
          {t("common.notFound")}
        </div>
      ) : (
        <div className="space-y-6">
          {/* Tab Navigation */}
          <div className="flex gap-2 border-b border-border-default pb-2">
            <Button
              variant={viewMode === "overview" ? "default" : "ghost"}
              size="sm"
              onClick={() => setViewMode("overview")}
            >
              {t("skills.detail.tab.overview")}
            </Button>
            <Button
              variant={viewMode === "files" ? "default" : "ghost"}
              size="sm"
              onClick={() => setViewMode("files")}
            >
              {t("skills.detail.tab.files")}
              <Badge variant="secondary" className="ml-1.5 text-xs">
                {skillDetail.files.length}
              </Badge>
            </Button>
            {selectedFile && (
              <Button
                variant={viewMode === "file-content" ? "default" : "ghost"}
                size="sm"
                onClick={() => setViewMode("file-content")}
              >
                <FileText size={12} className="mr-1" />
                {selectedFile.name}
              </Button>
            )}
          </div>

          {/* Overview Tab */}
          {viewMode === "overview" && (
            <div className="space-y-4">
              {/* Metadata Section */}
              <div className="rounded-lg border border-border-default p-4 space-y-3">
                <h3 className="text-sm font-medium">
                  {t("skills.detail.section.metadata")}
                </h3>

                <div className="grid grid-cols-2 gap-4 text-sm">
                  <div>
                    <span className="text-muted-foreground">
                      {t("skills.detail.field.directory")}:
                    </span>
                    <span className="ml-2 font-mono text-xs bg-muted px-1.5 py-0.5 rounded">
                      {skillDetail.directory}
                    </span>
                  </div>

                  {skillDetail.repoOwner && skillDetail.repoName && (
                    <div>
                      <span className="text-muted-foreground">
                        {t("skills.detail.field.source")}:
                      </span>
                      <span className="ml-2">
                        {skillDetail.repoOwner}/{skillDetail.repoName}
                        {skillDetail.repoBranch && `@${skillDetail.repoBranch}`}
                      </span>
                    </div>
                  )}

                  <div>
                    <span className="text-muted-foreground">
                      {t("skills.detail.field.installedAt")}:
                    </span>
                    <span className="ml-2">{formatDate(skillDetail.installedAt)}</span>
                  </div>

                  {skillDetail.updatedAt > 0 && (
                    <div>
                      <span className="text-muted-foreground">
                        {t("skills.detail.field.updatedAt")}:
                      </span>
                      <span className="ml-2">{formatDate(skillDetail.updatedAt)}</span>
                    </div>
                  )}

                  {skillDetail.contentHash && (
                    <div className="col-span-2">
                      <span className="text-muted-foreground">
                        {t("skills.detail.field.contentHash")}:
                      </span>
                      <code className="ml-2 text-xs bg-muted px-1.5 py-0.5 rounded break-all">
                        {skillDetail.contentHash}
                      </code>
                    </div>
                  )}
                </div>

                {/* App Status */}
                {enabledApps.length > 0 && (
                  <div className="flex items-center gap-2 flex-wrap">
                    <span className="text-sm text-muted-foreground">
                      {t("skills.detail.field.enabledApps")}:
                    </span>
                    {enabledApps.map((app) => (
                      <Badge key={app} variant="outline" className="text-xs">
                        {app}
                      </Badge>
                    ))}
                  </div>
                )}

                {/* External Link */}
                {skillDetail.readmeUrl && (
                  <div>
                    <span className="text-sm text-muted-foreground">
                      {t("skills.detail.field.documentation")}:
                    </span>
                    <a
                      href={skillDetail.readmeUrl}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="ml-2 inline-flex items-center gap-1 text-sm text-primary hover:underline"
                    >
                      {t("skills.detail.viewOnGithub")}
                      <ExternalLink size={12} />
                    </a>
                  </div>
                )}
              </div>

              {/* Description */}
              {skillDetail.description && (
                <div className="rounded-lg border border-border-default p-4">
                  <h3 className="text-sm font-medium mb-2">
                    {t("skills.detail.section.description")}
                  </h3>
                  <p className="text-sm text-muted-foreground">
                    {skillDetail.description}
                  </p>
                </div>
              )}

              {/* Manifest Preview */}
              {skillDetail.manifestContent && (
                <div className="rounded-lg border border-border-default p-4 space-y-3">
                  <div className="flex items-center justify-between">
                    <h3 className="text-sm font-medium">
                      {t("skills.detail.section.manifest")}
                    </h3>
                    <div className="flex gap-1">
                      <Button
                        variant={
                          manifestViewMode === "rendered" ? "secondary" : "ghost"
                        }
                        size="sm"
                        className="h-6 text-xs"
                        onClick={() => setManifestViewMode("rendered")}
                      >
                        {t("skills.detail.toggleRendered")}
                      </Button>
                      <Button
                        variant={
                          manifestViewMode === "source" ? "secondary" : "ghost"
                        }
                        size="sm"
                        className="h-6 text-xs"
                        onClick={() => setManifestViewMode("source")}
                      >
                        {t("skills.detail.toggleSource")}
                      </Button>
                    </div>
                  </div>
                  <MarkdownEditor
                    value={skillDetail.manifestContent}
                    readOnly={manifestViewMode === "rendered"}
                    className="max-h-96"
                  />
                </div>
              )}
            </div>
          )}

          {/* Files Tab */}
          {viewMode === "files" &&
            (skillDetail.files.length === 0 ? (
              <div className="rounded-lg border border-border-default p-8 text-center text-sm text-muted-foreground">
                {t("skills.detail.noFiles")}
              </div>
            ) : (
              <div className="rounded-lg border border-border-default overflow-hidden">
                <div className="p-2 bg-muted/30 border-b border-border-default">
                  <span className="text-xs text-muted-foreground">
                    {skillDetail.files.length} {t("skills.detail.files")}
                  </span>
                </div>
                <div className="p-2 max-h-[60vh] overflow-y-auto">
                  {renderFileTree(fileTree)}
                </div>
              </div>
            ))}

          {/* File Content Tab */}
          {viewMode === "file-content" && selectedFile && (
            <div className="space-y-4">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <FileText size={16} className="text-muted-foreground" />
                  <span className="font-medium">{selectedFile.name}</span>
                  <span className="text-sm text-muted-foreground">
                    ({formatFileSize(selectedFile.sizeBytes)})
                  </span>
                </div>
                <div className="flex items-center gap-2">
                  {isEditing ? (
                    <>
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={handleCancelEdit}
                        disabled={writeFileMutation.isPending}
                      >
                        <X size={14} className="mr-1" />
                        {t("common.cancel")}
                      </Button>
                      <Button
                        variant="default"
                        size="sm"
                        onClick={handleSaveFile}
                        disabled={
                          writeFileMutation.isPending ||
                          editedContent === fileContent
                        }
                      >
                        {writeFileMutation.isPending ? (
                          <Loader2 size={14} className="animate-spin mr-1" />
                        ) : (
                          <Save size={14} className="mr-1" />
                        )}
                        {t("skills.detail.save")}
                      </Button>
                    </>
                  ) : (
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => setIsEditing(true)}
                    >
                      <Pencil size={14} className="mr-1" />
                      {t("skills.detail.edit")}
                    </Button>
                  )}
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => setViewMode("files")}
                  >
                    <ChevronRight size={14} className="mr-1" />
                    {t("skills.detail.backToFiles")}
                  </Button>
                </div>
              </div>

              {isLoadingFile ? (
                <div className="flex items-center justify-center py-12">
                  <Loader2 size={24} className="animate-spin mr-2" />
                  <span>{t("common.loading")}</span>
                </div>
              ) : fileContent === null ? (
                <div className="text-center py-12 text-muted-foreground">
                  {t("skills.detail.fileNotFound")}
                </div>
              ) : isMarkdownFile(selectedFile.name) ? (
                <div className="rounded-lg border border-border-default p-4">
                  <MarkdownEditor
                    value={isEditing ? editedContent ?? "" : fileContent}
                    onChange={isEditing ? setEditedContent : undefined}
                    readOnly={!isEditing}
                  />
                </div>
              ) : (
                <div className="rounded-lg border border-border-default overflow-hidden">
                  {isEditing ? (
                    <textarea
                      className="w-full p-4 text-sm font-mono overflow-x-auto min-h-[400px] bg-background border-0 focus:ring-0 resize-y"
                      value={editedContent ?? ""}
                      onChange={(e) => setEditedContent(e.target.value)}
                    />
                  ) : (
                    <pre className="p-4 text-sm font-mono overflow-x-auto max-h-[60vh] whitespace-pre-wrap">
                      {fileContent}
                    </pre>
                  )}
                </div>
              )}
            </div>
          )}
        </div>
      )}
    </FullScreenPanel>
  );
};