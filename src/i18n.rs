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
        vec![Language::English, Language::Japanese]
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
        self.add("app_title", "CicadaGallery", "CicadaGallery", "CicadaGallery");
        self.add("search", "Search", "検索", "搜索");
        self.add("options", "Options", "オプション", "选项");
        self.add("close", "Close", "閉じる", "关闭");
        self.add("cancel", "Cancel", "キャンセル", "取消");
        self.add("ok", "OK", "OK", "确定");
        self.add("save", "Save", "保存", "保存");
        
        // View modes
        self.add("grid_view", "Grid View", "グリッド表示", "网格视图");
        self.add("list_view", "List View", "リスト表示", "列表视图");
        
        // Sort
        self.add("sort", "Sort:", "並び順:", "排序:");
        self.add("sort_added_date", "Created Date", "作成日時", "创建日期");
        self.add("sort_added_date_asc", "Created Date ↑", "作成日時 ↑", "创建日期 ↑");
        self.add("sort_added_date_desc", "Created Date ↓", "作成日時 ↓", "创建日期 ↓");
        self.add("sort_filename", "File Name", "ファイル名", "文件名");
        self.add("sort_filename_asc", "File Name ↑", "ファイル名 ↑", "文件名 ↑");
        self.add("sort_filename_desc", "File Name ↓", "ファイル名 ↓", "文件名 ↓");
        self.add("sort_duration", "Duration", "動画時間", "视频时长");
        self.add("sort_duration_asc", "Duration ↑", "動画時間 ↑", "视频时长 ↑");
        self.add("sort_duration_desc", "Duration ↓", "動画時間 ↓", "视频时长 ↓");
        
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
        self.add("folder_management", "Folder Management", "フォルダ管理", "文件夹管理");
        self.add("shader_management", "Shader Management", "シェーダー管理", "着色器管理");
        self.add("management", "Management", "管理", "管理");
        self.add("manage_folders", "Manage Folders...", "フォルダを管理...", "管理文件夹...");
        self.add("manage_shaders", "Manage Shaders...", "シェーダーを管理...", "管理着色器...");
        self.add("select_shader_to_use", "Select shader to use:", "使用するシェーダーを選択:", "选择要使用的着色器:");
        self.add("registered_folders", "Registered Folders:", "登録されているフォルダ:", "已注册的文件夹:");
        self.add("new_folder_name", "New folder name:", "新しいフォルダ名:", "新文件夹名:");
        self.add("add_folder_name", "Add Folder", "フォルダを追加", "添加文件夹");
        self.add("select_all", "Select All", "全て選択", "全选");
        self.add("clear_selection_count", "Clear Selection ({})", "選択解除 ({})", "清除选择 ({})");
        self.add("add_tag_to_selected", "Add Tag to Selected", "選択した動画にタグ追加", "为所选项添加标签");
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
        self.add("framerate_label", "Frame rate: {} fps", "フレームレート: {} fps", "帧率: {} fps");
        self.add("file_size", "File Size", "ファイルサイズ", "文件大小");
        self.add("size_gb", "Size: {:.2} GB", "サイズ: {:.2} GB", "大小: {:.2} GB");
        self.add("size_mb", "Size: {:.1} MB", "サイズ: {:.1} MB", "大小: {:.1} MB");
        self.add("folder", "Folder", "フォルダ", "文件夹");
        self.add("folder_label", "Folder: {}", "フォルダ: {}", "文件夹: {}");
        self.add("tags_label", "Tags: {}", "タグ: {}", "标签: {}");
        self.add("added_date", "Created Date", "作成日時", "创建日期");
        self.add("added_label", "Created: {}", "作成: {}", "创建: {}");
        self.add("last_played", "Last Played", "最終再生", "上次播放");
        self.add("last_played_label", "Last Played: {}", "最終再生: {}", "上次播放: {}");
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
        
        // Tag management
        self.add("manage_tags", "Manage Tags...", "タグを管理...", "管理标签...");
        self.add("tag_management", "Tag Management", "タグ管理", "标签管理");
        self.add("registered_tags", "Registered Tags:", "登録されているタグ:", "已注册的标签:");
        self.add("confirm_tag_delete_title", "Delete Tag", "タグを削除", "删除标签");
        self.add("confirm_tag_delete", "Are you sure you want to remove this tag?", "このタグを削除してもよろしいですか？", "确定要删除此标签吗？");
        self.add("tag_used_in_videos", "This tag is used in {} video(s).", "このタグは{}件の動画で使用されています。", "此标签在{}个视频中使用。");
        self.add("tag_will_be_removed", "The tag will be removed from all videos.", "タグは全ての動画から削除されます。", "标签将从所有视频中删除。");
        
        // Folder deletion
        self.add("confirm_folder_delete_title", "Delete Folder", "フォルダを削除", "删除文件夹");
        self.add("confirm_folder_delete", "Are you sure you want to remove this folder?", "このフォルダを削除してもよろしいですか？", "确定要删除此文件夹吗？");
        self.add("folder_contains_videos", "This folder contains {} video(s).", "このフォルダには{}件の動画があります。", "此文件夹包含{}个视频。");
        self.add("delete_videos_too", "Also delete video profiles", "動画のプロファイルも削除する", "同时删除视频配置");
        self.add("keep_videos", "Keep video profiles", "動画のプロファイルを残す", "保留视频配置");
        self.add("folder_only", "Remove folder only", "フォルダのみ削除", "仅删除文件夹");
        
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
        
        // Premium features
        self.add("scene_thumbnails_locked", "🔒 Scene Thumbnails", "🔒 シーンサムネイル", "🔒 场景缩略图");
        self.add("premium_feature_available", "This feature is available in Premium version", "この機能はプレミアム版で利用可能です", "此功能在高级版中可用");
        self.add("premium_features", "Premium features:", "プレミアム機能:", "高级功能:");
        self.add("premium_scene_generation", "• Scene thumbnail generation", "• シーンサムネイル生成", "• 场景缩略图生成");
        self.add("premium_star_ratings", "• 1-5 star ratings", "• 1-5星評価", "• 1-5星评分");
        self.add("premium_glsl_shaders", "• GLSL shaders", "• GLSLシェーダー", "• GLSL着色器");
        self.add("premium_gpu_rendering", "• GPU high-quality rendering", "• GPU高品質レンダリング", "• GPU高质量渲染");
        self.add("premium_unlimited_storage", "• Unlimited video storage", "• 無制限の動画プロファイル", "• 无限视频存储");
        self.add("premium_multi_select", "• Multi-select for folders/tags", "• フォルダ/タグの複数選択", "• 文件夹/标签多选");
        
        // Premium promotion
        self.add("premium_promotion_title", "🌟 Upgrade to Premium", "🌟 プレミアム版にアップグレード", "🌟 升级到高级版");
        self.add("premium_limit_reached", "You've reached the free tier limit of 100 videos.", "無償版の上限（100本）に達しました。", "您已达到免费版的100个视频上限。");
        self.add("premium_unlock_features", "Upgrade to Premium to unlock:", "プレミアム版で以下の機能をアンロック:", "升级到高级版以解锁:");
        self.add("premium_how_to_upgrade", "To upgrade, edit settings.json and set \"is_premium\": true", "アップグレードするには、settings.jsonを編集して \"is_premium\": true に設定してください", "要升级，请编辑settings.json并设置 \"is_premium\": true");
        self.add("premium_settings_location", "Settings file location:", "設定ファイルの場所:", "设置文件位置:");
        self.add("got_it", "Got it!", "了解しました！", "明白了！");
        
        // License activation
        self.add("enter_license_key", "Enter License Key", "ライセンスキーを入力", "输入许可证密钥");
        self.add("activate_license", "Activate License", "ライセンスを有効化", "激活许可证");
        self.add("license_key_label", "License Key:", "ライセンスキー:", "许可证密钥:");
        self.add("paste_license_key", "Paste your license key here", "ライセンスキーを貼り付けてください", "在此粘贴您的许可证密钥");
        self.add("activate", "Activate", "有効化", "激活");
        self.add("license_info", "License Information", "ライセンス情報", "许可证信息");
        self.add("issued_to", "Issued to:", "発行先:", "发给:");
        self.add("expires", "Expires:", "有効期限:", "到期:");
        self.add("never_expires", "Never", "無期限", "永不过期");
        self.add("license_status", "Status:", "ステータス:", "状态:");
        self.add("view_license", "View License", "ライセンス情報", "查看许可证");
        
        // Premium purchase promotion
        self.add("premium_benefits_title", "[Premium Benefits]", "[プレミアム版の特典]", "[高级版特权]");
        self.add("premium_benefit_1", "* 5-star rating system", "* 5段階評価システム", "* 5星评分系统");
        self.add("premium_benefit_2", "* Multi-select folders & tags", "* フォルダ・タグの複数選択", "* 多选文件夹和标签");
        self.add("premium_benefit_3", "* GPU high-quality rendering", "* GPU高画質レンダリング", "* GPU高画质渲染");
        self.add("premium_benefit_4", "* Custom GLSL shaders", "* カスタムGLSLシェーダー", "* 自定义GLSL着色器");
        self.add("premium_benefit_5", "* Unlimited video profiles", "* 無制限の動画プロファイル", "* 无限视频配置");
        self.add("purchase_premium", "Purchase Premium", "プレミアム版を購入", "购买高级版");
        
        // Free tier scene limit
        self.add("free_tier_scene_limit", "(Free: up to 5 scenes)", "(無料版: 最大5シーンまで)", "(免费版：最多5个场景)");
        self.add("free_tier_scene_limit_reached", "Free tier: 5 scenes limit", "無料版: 5シーンまで", "免费版：5个场景限制");
        self.add("premium_unlimited_scenes", "Upgrade to Premium for unlimited scene thumbnails!", "プレミアム版で無制限のシーンサムネイル!", "升级到高级版获取无限场景缩略图！");
    }
    
    fn add(&mut self, key: &str, en: &str, ja: &str, zh: &str) {
        let mut translations = HashMap::new();
        translations.insert(Language::English, en.to_string());
        translations.insert(Language::Japanese, ja.to_string());
        translations.insert(Language::Chinese, zh.to_string());
        self.translations.insert(key.to_string(), translations);
    }
}
