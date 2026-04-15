import {createApp} from "vue";
import App from "./App.vue";
import ElementPlus from 'element-plus';
import 'element-plus/dist/index.css'
import i18n from "./i18n.ts";
import router from "./router.ts";

createApp(App).use(router).use(ElementPlus).use(i18n).mount("#app");
