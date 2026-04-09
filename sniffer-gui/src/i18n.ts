import { createI18n } from "vue-i18n";

// 导入语言资源文件
import zhCN from "./locales/zh-CN.json";

// 类型定义
type MessageSchema = typeof zhCN;

const i18n = createI18n<[MessageSchema], 'zh' | 'en'>({
    legacy: false, // 使用Composition API风格
    locale: 'zh', // 默认语言
    fallbackLocale: 'zh', // 备用语言
    messages: {
        zh: zhCN,
        en: zhCN,
    },
    // 支持嵌套路径访问
    allowComposition: true,
});

export default i18n;