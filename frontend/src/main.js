import { createApp } from "vue";
import * as VueRouter from "vue-router";
import App from "./App.vue";
import Blocks from "./routes/Blocks.vue";
import Transactions from "./routes/Transactions.vue";
import Validators from "./routes/Validators.vue";
import Stats from "./routes/Stats.vue";
import "./style.css";

const routes = [
  { path: "/", component: Stats },
  { path: "/blocks", component: Blocks },
  { path: "/transactions", component: Transactions },
  { path: "/validators", component: Validators },
];

const router = VueRouter.createRouter({
  history: VueRouter.createWebHashHistory(),
  routes,
});

const app = createApp(App);
app.use(router);
app.mount("#app");
