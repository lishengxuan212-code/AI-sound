import { createApp } from 'vue';
import App from './App.vue';
import { bootstrapApp } from './app/bootstrap';
import './styles.css';

createApp(App).mount('#app');
void bootstrapApp();
