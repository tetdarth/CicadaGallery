use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Language {
    English,
    Japanese,
    Chinese,
}

impl Language {
    pub fn all() -> Vec<Language> {
        vec![Language::English, Language::Japanese, Language::Chinese]
    }
    
    pub fn name(&self) -> &'static str {
        match self {
            Language::English => "English",
            Language::Japanese => "日本語",
            Language::Chinese => "中文",
        }
    }
}

impl Default for Language {
    fn default() -> Self {
        Language::English
    }
}

/// Internationalization text manager
pub struct I18n {
    current_language: Language,
    translations: HashMap<String, HashMap<Language, String>>,
}

impl I18n {
    pub fn new(language: Language) -> Self {
        let mut i18n = Self {
            current_language: language,
            translations: HashMap::new(),
        };
        i18n.load_translations();
        i18n
    }
    
    pub fn set_language(&mut self, language: Language) {
        self.current_language = language;
    }
    
    pub fn get_language(&self) -> Language {
        self.current_language
    }
    
    pub fn t(&self, key: &str) -> String {
        self.translations
            .get(key)
            .and_then(|translations| translations.get(&self.current_language).cloned())
            .unwrap_or_else(|| key.to_string())
    }
    
    fn load_translations(&mut self) {
        // UI - General
        self.add("app_title", "Cicada Gallery", "Cicada Gallery", "Cicada Gallery");
        self.add("search", "Search", "検索", "搜索");
        self.add("options", "Options", "オプション", "选项");
        self.add("close", "Close", "閉じる", "关闭");
        self.add("cancel", "Cancel", "キャンセル", "取消");
        self.add("ok", "OK", "OK", "确定");
        self.add("save", "Save", "保存", "保存");
        
        // View modes
        self.add("grid_view", "Grid View", "グリッド表示", "网格视图");
        self.add("list_view", "List View", "リスト表示", "列表视图");
        
        // Filters
        self.add("filters", "Filters", "フィルター", "筛选");
        self.add("folders", "Folders:", "フォルダ:", "文件夹:");
        self.add("tags_colon", "Tags:", "タグ:", "标签:");
        self.add("all", "All", "全て", "全部");
        self.add("all_folders", "All Folders", "全てのフォルダ", "所有文件夹");
        self.add("all_tags", "All Tags", "全てのタグ", "所有标签");
        self.add("favorites_only", "Favorites Only", "お気に入りのみ", "仅收藏");
        self.add("show_all", "Show All", "全て表示", "显示全部");
        self.add("total_videos", "Total Videos: {}", "動画総数: {}", "视频总数: {}");
        self.add("favorites_count", "Favorites: {}", "お気に入り: {}", "收藏数: {}");
        
        // Video operations
        self.add("add_videos", "Add Videos", "動画を追加", "添加视频");
        self.add("add_folder", "Add Folder", "フォルダを追加", "添加文件夹");
        self.add("rescan_folders", "Rescan Folders", "フォルダを再スキャン", "重新扫描文件夹");
        self.add("select_all", "Select All", "全て選択", "全选");
        self.add("clear_selection_count", "Clear Selection ({})", "選択解除 ({})", "清除选择 ({})");
        self.add("play_video", "Play Video", "動画を再生", "播放视频");
        self.add("delete", "Delete", "削除", "删除");
        self.add("delete_selected", "Delete Selected", "選択項目を削除", "删除所选项");
        self.add("show_in_folder", "Show in Folder", "フォルダで表示", "在文件夹中显示");
        self.add("toggle_favorite", "Toggle Favorite", "お気に入り切替", "切换收藏");
        self.add("clear_selection", "Clear Selection", "選択解除", "清除选择");
        
        // Video details
        self.add("video_details", "Video Details", "動画の詳細", "视频详情");
        self.add("selected_video", "Selected Video", "選択中の動画", "已选择视频");
        self.add("no_thumbnail", "No Thumbnail", "サムネイルなし", "无缩略图");
        self.add("title", "Title", "タイトル", "标题");
        self.add("path", "Path", "パス", "路径");
        self.add("duration", "Duration", "再生時間", "时长");
        self.add("duration_label", "Duration: {}", "再生時間: {}", "时长: {}");
        self.add("resolution", "Resolution", "解像度", "分辨率");
        self.add("resolution_label", "Resolution: {}", "解像度: {}", "分辨率: {}");
        self.add("file_size", "File Size", "ファイルサイズ", "文件大小");
        self.add("size_gb", "Size: {:.2} GB", "サイズ: {:.2} GB", "大小: {:.2} GB");
        self.add("size_mb", "Size: {:.1} MB", "サイズ: {:.1} MB", "大小: {:.1} MB");
        self.add("folder", "Folder", "フォルダ", "文件夹");
        self.add("folder_label", "Folder: {}", "フォルダ: {}", "文件夹: {}");
        self.add("tags_label", "Tags: {}", "タグ: {}", "标签: {}");
        self.add("added_date", "Added Date", "追加日時", "添加日期");
        self.add("added_label", "Added: {}", "追加: {}", "添加: {}");
        self.add("last_played", "Last Played", "最終再生", "上次播放");
        self.add("last_played_label", "Last Played: {}", "最終再生: {}", "上次播放: {}");
        self.add("rating", "Rating", "評価", "评分");
        self.add("rating_filter", "Rating Filter", "評価フィルター", "评分筛选");
        self.add("min_rating", "Minimum Rating:", "最低評価:", "最低评分:");
        self.add("favorite", "Favorite", "お気に入り", "收藏");
        self.add("add_to_favorites", "☆ Add to Favorites", "☆ お気に入りに追加", "☆ 添加到收藏");
        self.add("remove_from_favorites", "★ Remove from Favorites", "★ お気に入りから削除", "★ 从收藏中移除");
        self.add("never", "Never", "未再生", "从未播放");
        
        // Tags
        self.add("tags", "Tags", "タグ", "标签");
        self.add("add_tag", "Add Tag", "タグを追加", "添加标签");
        self.add("remove_tag", "Remove Tag", "タグを削除", "删除标签");
        self.add("create_tag", "Create", "作成", "创建");
        self.add("existing_tags", "Existing Tags", "既存のタグ", "现有标签");
        self.add("create_new_tag", "Or create new tag:", "または新しいタグを作成:", "或创建新标签:");
        self.add("select_or_create_tag", "Select existing tag or create new:", "既存のタグを選択または新規作成:", "选择现有标签或创建新标签:");
        
        // Scenes
        self.add("scene_thumbnails", "Scene Thumbnails", "シーンサムネイル", "场景缩略图");
        self.add("generate_scenes", "Generate Scene Thumbnails", "シーンサムネイルを生成", "生成场景缩略图");
        self.add("no_scenes_yet", "No scenes detected yet.", "まだシーンが検出されていません。", "尚未检测到场景。");
        self.add("play_from_scene", "Play from Scene", "シーンから再生", "从场景播放");
        self.add("delete_scene", "Delete Scene", "シーンを削除", "删除场景");
        self.add("selected_count", "{} selected", "{}個選択中", "已选择{}个");
        
        // Options/Settings
        self.add("display_settings", "Display Settings", "表示設定", "显示设置");
        self.add("thumbnail_scale", "Thumbnail Scale", "サムネイルのサイズ", "缩略图大小");
        self.add("show_full_filename", "Show full filename in grid view", "グリッド表示でファイル名を全て表示", "在网格视图中显示完整文件名");
        self.add("show_tags_in_grid", "Show tags in grid view", "グリッド表示でタグを表示", "在网格视图中显示标签");
        self.add("theme", "Theme", "テーマ", "主题");
        self.add("dark_mode", "Dark Mode", "ダークモード", "深色模式");
        self.add("light_mode", "Light Mode", "ライトモード", "浅色模式");
        self.add("language", "Language", "言語", "语言");
        
        // Player settings
        self.add("player_settings", "Player Settings", "プレイヤー設定", "播放器设置");
        self.add("always_on_top", "Keep player window always on top", "プレイヤーを常に最前面に表示", "播放器窗口始终置顶");
        self.add("use_gpu_hq", "Use GPU high-quality rendering (profile=gpu-hq)", "GPU高品質レンダリング (profile=gpu-hq)", "使用GPU高质量渲染 (profile=gpu-hq)");
        self.add("use_custom_shaders", "Use custom GLSL shaders", "カスタムGLSLシェーダーを使用", "使用自定义GLSL着色器");
        self.add("select_shader", "Select shader:", "シェーダーを選択:", "选择着色器:");
        self.add("no_shader", "No shader", "シェーダーなし", "无着色器");
        self.add("use_frame_interpolation", "Use frame interpolation (motion smoothing)", "フレーム補間を使用 (モーション補間)", "使用帧插值 (运动平滑)");
        
        // Confirmation dialogs
        self.add("confirm_delete", "Are you sure you want to delete this video from the gallery?", "このギャラリーから動画を削除してもよろしいですか？", "确定要从图库中删除此视频吗？");
        self.add("confirm_delete_video", "Are you sure you want to delete this video?", "この動画を削除してもよろしいですか？", "确定要删除此视频吗？");
        self.add("confirm_delete_videos", "Are you sure you want to delete {} selected videos?", "選択された{}個の動画を削除してもよろしいですか？", "确定要删除{}个所选视频吗？");
        self.add("delete_video", "Delete Video", "動画を削除", "删除视频");
        self.add("delete_selected_videos", "Delete Selected Videos", "選択した動画を削除", "删除所选视频");
        self.add("delete_keep_cache", "Delete (Keep Cache)", "削除 (キャッシュを保持)", "删除（保留缓存）");
        self.add("delete_remove_all", "Delete (Remove All)", "削除 (全て削除)", "删除（全部删除）");
        self.add("title_label", "Title: {}", "タイトル: {}", "标题: {}");
        self.add("file_will_not_be_deleted", "(The actual file will not be deleted)", "（実際のファイルは削除されません）", "（实际文件不会被删除）");
        self.add("yes_delete", "Yes, Delete", "はい、削除します", "是的，删除");
        
        // Status messages
        self.add("no_videos_found", "No videos found", "動画が見つかりません", "未找到视频");
        self.add("add_videos_to_start", "Add videos to get started", "動画を追加して開始", "添加视频以开始");
        self.add("video_count", "{} videos", "{}個の動画", "{}个视频");
        
        // Tooltips
        self.add("click_play_ctrl_select", "Click: Play | Ctrl+Click: Select | Shift+Click: Range select", "クリック: 再生 | Ctrl+クリック: 選択 | Shift+クリック: 範囲選択", "点击：播放 | Ctrl+点击：选择 | Shift+点击：范围选择");
        self.add("right_click_options", "Right-click for options", "右クリックでオプション", "右键单击查看选项");
    }
    
    fn add(&mut self, key: &str, en: &str, ja: &str, zh: &str) {
        let mut translations = HashMap::new();
        translations.insert(Language::English, en.to_string());
        translations.insert(Language::Japanese, ja.to_string());
        translations.insert(Language::Chinese, zh.to_string());
        self.translations.insert(key.to_string(), translations);
    }
}
