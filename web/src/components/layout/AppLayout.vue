<script setup lang="ts">
import { ref, watch } from 'vue'
import { useRoute } from 'vue-router'
const menuOpen = ref(false)
const route = useRoute()
watch(() => route.path, () => { menuOpen.value = false })
import Sidebar from './Sidebar.vue'
import TopBar from './TopBar.vue'
</script>

<template>
  <div class="flex flex-col md:flex-row min-h-screen md:h-screen bg-gray-50 dark:bg-rd-scaffold">
    <Sidebar :class="menuOpen ? 'flex' : 'hidden md:flex'" id="app-navigation" />
    <div class="flex-1 min-w-0 flex flex-col overflow-hidden">
      <TopBar>
        <template #navigation>
          <button class="ui-button md:hidden" aria-controls="app-navigation" :aria-expanded="menuOpen" @click="menuOpen = !menuOpen">Menu</button>
        </template>
      </TopBar>
      <main class="flex-1 overflow-y-auto p-4 md:p-6">
        <router-view />
      </main>
    </div>
  </div>
</template>
