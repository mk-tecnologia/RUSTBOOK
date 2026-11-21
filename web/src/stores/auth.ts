import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { useAddressBookStore } from './addressBook'
import * as authApi from '../api/auth'

export const useAuthStore = defineStore('auth', () => {
  const token = ref(localStorage.getItem('token') || '')
  const user = ref<authApi.UserInfo | null>(null)

  const isAuthenticated = computed(() => !!token.value)
  const isAdmin = computed(() => user.value?.is_admin ?? false)

  async function login(username: string, password: string) {
    useAddressBookStore().reset()
    const res = await authApi.login(username, password)
    token.value = res.access_token
    user.value = res.user
    localStorage.setItem('token', res.access_token)
    localStorage.setItem('user', JSON.stringify(res.user))
  }

  async function logout() {
    try {
      await authApi.logout()
    } catch {
      // ignore
    }
    clearSession()
  }

  function clearSession() {
    useAddressBookStore().reset()
    token.value = ''
    user.value = null
    localStorage.removeItem('token')
    localStorage.removeItem('user')
  }

  async function refresh() {
    if (!token.value) return
    user.value = await authApi.getCurrentUser()
  }
  return { token, user, isAuthenticated, isAdmin, login, logout, refresh, clearSession }
})
