<script setup lang="ts">
import { ref } from 'vue'
import type { UserItem } from '../../api/addressBook'
import { userDraft, userPayload } from '../../utils/userEditor'
import BaseDialog from '../ui/BaseDialog.vue'
import ErrorNotice from '../ui/ErrorNotice.vue'
const props = defineProps<{ user?: UserItem; busy: boolean; error: string }>()
const emit = defineEmits<{ close: []; save: [payload: ReturnType<typeof userPayload>] }>()
const draft = ref(userDraft(props.user))
const validation = ref('')
function submit() {
  if (props.busy) return
  validation.value = ''
  try { emit('save', userPayload(draft.value, props.user)) }
  catch (error) { validation.value = error instanceof Error ? error.message : 'Check the form.' }
}
</script>
<template>
  <BaseDialog :title="user ? 'Edit user' : 'New user'" :busy="busy" @close="emit('close')">
    <form class="space-y-4" @submit.prevent="submit">
      <ErrorNotice :message="validation || error" />
      <fieldset :disabled="busy" class="grid grid-cols-1 gap-4 sm:grid-cols-2">
        <div class="sm:col-span-2"><label for="user-username" class="mb-1 block text-xs font-medium">Username *</label><input id="user-username" v-model="draft.username" :readonly="Boolean(user)" required maxlength="128" autocomplete="off" class="peer-input"><p v-if="user" class="mt-1 text-xs text-gray-500 dark:text-rd-text-secondary">The login username is fixed.</p></div>
        <div><label for="user-name" class="mb-1 block text-xs font-medium">Display name</label><input id="user-name" v-model="draft.name" maxlength="128" autocomplete="name" class="peer-input"></div>
        <div><label for="user-email" class="mb-1 block text-xs font-medium">Email</label><input id="user-email" v-model="draft.email" type="email" maxlength="254" autocomplete="email" class="peer-input"></div>
        <div><label for="user-password" class="mb-1 block text-xs font-medium">{{ user ? 'New password' : 'Password *' }}</label><input id="user-password" v-model="draft.password" type="password" :required="!user" minlength="12" maxlength="1024" autocomplete="new-password" class="peer-input"></div>
        <div><label for="user-confirmation" class="mb-1 block text-xs font-medium">Confirm password{{ user ? '' : ' *' }}</label><input id="user-confirmation" v-model="draft.confirmation" type="password" :required="!user || Boolean(draft.password)" maxlength="1024" autocomplete="new-password" class="peer-input"></div>
        <p class="sm:col-span-2 text-xs text-gray-500 dark:text-rd-text-secondary">{{ user ? 'Leave both password fields empty to keep the current password. Changing the password signs out existing sessions.' : 'Use at least 12 characters and repeat the same password to confirm.' }}</p>
        <label class="flex items-center gap-2 text-sm"><input v-model="draft.is_admin" type="checkbox" class="rounded text-rd-primary">Administrator</label>
        <label v-if="user" class="flex items-center gap-2 text-sm"><input v-model="draft.status" type="checkbox" :true-value="1" :false-value="0" class="rounded text-rd-primary">Active account</label>
      </fieldset>
      <div class="flex justify-end gap-2 border-t border-gray-100 pt-4 dark:border-rd-border"><button type="button" :disabled="busy" class="ui-button" @click="emit('close')">Cancel</button><button type="submit" :disabled="busy" class="rounded-lg bg-rd-primary px-4 py-2 text-sm font-medium text-white hover:bg-rd-primary-hover disabled:opacity-50">{{ busy ? 'Saving…' : user ? 'Save changes' : 'Create user' }}</button></div>
    </form>
  </BaseDialog>
</template>
