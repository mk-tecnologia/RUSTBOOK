<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import type { Tag } from '../../api/addressBook'
import { tagHex } from '../../utils/colors'
import { filterTags, selectTag, tagPopupPlacement } from '../../utils/tags'
const props = defineProps<{ id: string; tags: Tag[]; modelValue: string[]; multiple?: boolean; disabled?: boolean; placeholder?: string; label: string }>()
const emit = defineEmits<{ 'update:modelValue': [value: string[]] }>()
const opened = ref(false)
const query = ref('')
const active = ref(0)
const root = ref<HTMLElement>()
const input = ref<HTMLInputElement>()
const trigger = ref<HTMLButtonElement>()
const popup = ref<HTMLElement>()
const popupStyle = ref<Record<string, string>>({})
const matches = computed(() => filterTags(props.tags, query.value))
const options = computed(() => props.multiple || query.value.trim() ? matches.value : [{ name: '', color: 0 }, ...matches.value])
const selected = computed(() => props.modelValue.map(name => props.tags.find(tag => tag.name === name) || { name, color: 0xff0071ff }))
const summary = computed(() => selected.value.length ? `${selected.value[0]!.name}${selected.value.length > 1 ? ` +${selected.value.length - 1}` : ''}` : props.placeholder || 'Select tags…')
function position() {
  const rect = trigger.value?.getBoundingClientRect()
  if (!rect || !popup.value) return
  const vv = window.visualViewport
  const viewport = { left: vv?.offsetLeft || 0, top: vv?.offsetTop || 0, width: vv?.width || window.innerWidth, height: vv?.height || window.innerHeight }
  popup.value.style.width = `${Math.min(rect.width, viewport.width - 16)}px`
  const previousHeight = popup.value.style.maxHeight
  popup.value.style.maxHeight = 'none'
  const contentHeight = popup.value.scrollHeight
  popup.value.style.maxHeight = previousHeight
  const placement = tagPopupPlacement(rect, viewport, contentHeight)
  popupStyle.value = Object.fromEntries(Object.entries(placement).map(([key, value]) => [key, `${value}px`]))
}
async function open() {
  if (props.disabled) return
  opened.value = true; query.value = ''; active.value = 0
  await nextTick()
  if (!props.multiple) active.value = Math.max(0, options.value.findIndex(tag => tag.name === (props.modelValue[0] || '')))
  position(); await nextTick()
  popup.value?.showPopover?.()
  input.value?.focus({ preventScroll: true })
}
function close(restore = false) { if (typeof popup.value?.hidePopover === 'function' && popup.value.matches(':popover-open')) popup.value.hidePopover(); opened.value = false; if (restore) trigger.value?.focus() }
function choose(name: string) { emit('update:modelValue', selectTag(props.modelValue, name, Boolean(props.multiple))); if (!props.multiple) close(true) }
async function move(delta: number) {
  if (!options.value.length) return
  active.value = (active.value + delta + options.value.length) % options.value.length
  await nextTick()
  const option = root.value?.querySelector(`#${props.id}-option-${active.value}`) as HTMLElement | null
  const list = option?.parentElement
  if (option && list) {
    const itemBounds = option.getBoundingClientRect(); const listBounds = list.getBoundingClientRect()
    if (itemBounds.top < listBounds.top) list.scrollTop -= listBounds.top - itemBounds.top
    else if (itemBounds.bottom > listBounds.bottom) list.scrollTop += itemBounds.bottom - listBounds.bottom
  }
}
function escape(event: KeyboardEvent) { if (opened.value) { event.preventDefault(); event.stopPropagation(); close(true) } }
function keydown(event: KeyboardEvent) {
  if (event.key === 'ArrowDown' || event.key === 'ArrowUp') { event.preventDefault(); void move(event.key === 'ArrowDown' ? 1 : -1) }
  else if (event.key === 'Enter') { event.preventDefault(); const option = options.value[active.value]; if (option) choose(option.name) }
  else if (event.key === 'Escape') escape(event)
}
function outside(event: PointerEvent) { if (!root.value?.contains(event.target as Node)) close() }
function focusout(event: FocusEvent) { if (event.relatedTarget && !root.value?.contains(event.relatedTarget as Node)) close() }
function resize() { if (opened.value) position() }
function scroll(event: Event) { if (opened.value && !(event.target instanceof Node && popup.value?.contains(event.target))) close() }
watch(() => props.disabled, value => { if (value) close() })
watch(query, async () => { await nextTick(); if (opened.value) position() })
onMounted(() => { document.addEventListener('pointerdown', outside); window.addEventListener('resize', resize); window.addEventListener('scroll', scroll, true); window.visualViewport?.addEventListener('resize', resize) })
onBeforeUnmount(() => { document.removeEventListener('pointerdown', outside); window.removeEventListener('resize', resize); window.removeEventListener('scroll', scroll, true); window.visualViewport?.removeEventListener('resize', resize) })
</script>
<template>
  <div ref="root" class="relative min-w-0" @focusout="focusout">
    <button :id="id" ref="trigger" type="button" :disabled="disabled" :aria-label="label" aria-haspopup="listbox" :aria-expanded="opened" :aria-controls="`${id}-options`" class="peer-input flex h-10 w-full min-w-0 items-center gap-2 text-left disabled:opacity-50" @click="opened ? close() : open()" @keydown.down.prevent="open" @keydown.up.prevent="open" @keydown.esc="escape">
      <span v-if="selected.length" class="size-2 shrink-0 rounded-full" :style="{ backgroundColor: tagHex(selected[0]!.color) }" />
      <span class="min-w-0 flex-1 truncate" :title="selected.map(tag => tag.name).join(', ')">{{ summary }}</span>
      <svg aria-hidden="true" class="size-4 shrink-0 text-gray-400" fill="none" stroke="currentColor" stroke-width="1.6" viewBox="0 0 24 24"><path stroke-linecap="round" d="m6 9 6 6 6-6" /></svg>
    </button>
    <div v-if="opened" ref="popup" popover="manual" :style="popupStyle" class="fixed inset-auto z-50 m-0 flex flex-col overflow-hidden rounded-xl border border-gray-200 bg-white text-gray-900 shadow-xl dark:border-rd-border dark:bg-rd-card dark:text-rd-text">
      <div class="border-b border-gray-100 p-2 dark:border-rd-border"><input ref="input" v-model="query" type="search" role="combobox" :aria-label="`Search ${label.toLowerCase()}`" aria-autocomplete="list" aria-expanded="true" :aria-controls="`${id}-options`" :aria-activedescendant="options.length ? `${id}-option-${active}` : undefined" autocomplete="off" placeholder="Search tags…" class="peer-input" @input="active = 0" @keydown="keydown"></div>
      <div :id="`${id}-options`" role="listbox" :aria-label="label" :aria-multiselectable="Boolean(multiple)" class="min-h-0 max-h-56 flex-1 overflow-y-auto overscroll-contain p-1">
        <div v-for="(tag, index) in options" :id="`${id}-option-${index}`" :key="tag.name" role="option" :aria-selected="tag.name ? modelValue.includes(tag.name) : !modelValue.length" class="flex cursor-pointer items-center gap-2 rounded-lg px-3 py-2 text-sm" :class="active === index ? 'bg-rd-primary/10 text-rd-primary' : 'hover:bg-gray-100 dark:hover:bg-rd-card-hover'" @mouseenter="active = index" @mousedown.prevent @click="choose(tag.name)">
          <span v-if="multiple" aria-hidden="true" class="flex size-4 shrink-0 items-center justify-center rounded border" :class="modelValue.includes(tag.name) ? 'border-rd-primary bg-rd-primary text-white' : 'border-gray-300 dark:border-gray-500'"><svg v-if="modelValue.includes(tag.name)" class="size-3" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24"><path d="m5 12 4 4L19 6" /></svg></span>
          <span v-if="tag.name" class="size-2 shrink-0 rounded-full" :style="{ backgroundColor: tagHex(tag.color) }" /><span class="min-w-0 flex-1 break-words">{{ tag.name || placeholder || 'All tags' }}</span>
          <svg v-if="!multiple && (tag.name ? modelValue.includes(tag.name) : !modelValue.length)" aria-hidden="true" class="size-4 shrink-0" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24"><path d="m5 12 4 4L19 6" /></svg>
        </div>
        <p v-if="!matches.length" class="px-3 py-4 text-xs text-gray-500">{{ query ? 'No matching tags' : 'No tags yet' }}</p>
      </div>
      <div v-if="multiple" class="flex items-center justify-between border-t border-gray-100 px-3 py-2 text-xs text-gray-500 dark:border-rd-border"><span>{{ modelValue.length }} selected</span><button v-if="modelValue.length" type="button" class="text-rd-primary" @click="emit('update:modelValue', [])">Clear</button><button type="button" class="font-medium text-rd-primary" @click="close(true)">Done</button></div>
    </div>
  </div>
</template>
