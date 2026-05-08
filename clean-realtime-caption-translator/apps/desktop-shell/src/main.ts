import { createApp } from 'vue';
import App from './App.vue';
import { bootstrapApp } from './app/bootstrap';
import './styles.css';

void bootstrapApp();

createApp(App).mount('#app');
