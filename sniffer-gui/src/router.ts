import { createMemoryHistory, createRouter } from 'vue-router'

const routes = [
    {
        path: '/',
        redirect: '/log'
    },
    {
        path: '/index',
        name: 'index',
        component: () => import('./pages/index.vue')
    },
    {
        path: '/connection',
        name: 'connection',
        component: () => import('./pages/connection.vue')
    },
    {
        path: '/log',
        name: 'log',
        component: () => import('./pages/log.vue')
    },
]

const router = createRouter({
    history: createMemoryHistory(),
    routes,
});

export default router;