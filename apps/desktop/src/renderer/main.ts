import './assets/main.css'

import { createApp } from 'vue'
import App from './App.vue'
import { initializeAppearance } from './appearance'

// Initialize appearance before Vue mount
initializeAppearance()

createApp(App).mount('#app')
