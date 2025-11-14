/// 错误消息常量模块
/// 统一管理所有业务错误提示信息

// ==================== 试卷相关 ====================
pub const PAPER_TYPE_NOT_FOUND: &str = "试卷类型不存在";
pub const PAPER_NOT_FOUND: &str = "试卷未找到";

// ==================== 题目相关 ====================
pub const QUESTION_NOT_FOUND: &str = "题目不存在";
pub const QUESTION_PAPER_TYPE_REQUIRED: &str = "请指定试卷类型";

// ==================== 成语相关 ====================
pub const IDIOM_NOT_FOUND: &str = "成语未找到";
pub const INVALID_PAPER_TYPE: &str = "错误的试卷类型";

// ==================== 支付相关 ====================
pub const QRCODE_GENERATION_FAILED: &str = "支付码生成失败";
pub const ORDER_NOT_FOUND: &str = "订单不存在";

// ==================== BBS相关 ====================
pub const ISSUE_NOT_FOUND: &str = "没找到帖子";

// ==================== 用户相关 ====================
pub const USER_AVATAR_NOT_FOUND: &str = "用户头像不存在";

// ==================== 认证相关 ====================
pub const INVALID_COOKIE: &str = "invalid cookie";
pub const MISSING_TOKEN: &str = "Missing token";
pub const TOKEN_CREATION_ERROR: &str = "Token created error";

/// 生成无效 token 错误消息
pub fn invalid_token_msg(token: &str) -> String {
    format!("invalid token:{token}")
}
