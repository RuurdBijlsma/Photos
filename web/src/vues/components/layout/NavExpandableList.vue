<script setup lang="ts" generic="T extends { id: string }">
import { computed, ref } from 'vue'

const props = defineProps<{
  title: string
  to: string
  icon: string
  items: T[]
}>()

const expanded = defineModel<boolean>('expanded', { required: true })

const maxShown = ref(5)

const truncatedItems = computed(() => props.items.slice(0, maxShown.value))
const hasMore = computed(() => props.items.length > maxShown.value)
</script>

<template>
  <div class="expandable-list-wrapper">
    <div class="albums-nav">
      <v-list-item class="albums-nav-item" rounded :prepend-icon="icon" :to="to">
        <v-list-item-title>{{ title }}</v-list-item-title>
      </v-list-item>
      <v-btn
        @click="expanded = !expanded"
        class="albums-nav-btn"
        density="compact"
        icon="mdi-menu-down"
        v-if="items.length > 0"
        :class="{
          'point-down': expanded,
        }"
        variant="plain"
      ></v-btn>
    </div>

    <v-expand-transition v-if="items.length > 0">
      <div v-show="expanded" class="expandable-list-container">
        <div class="expandable-list-branch">
          <slot name="item" v-for="item in truncatedItems" :key="item.id" :item="item"></slot>

          <v-btn
            density="compact"
            variant="plain"
            rounded
            color="secondary"
            v-if="hasMore"
            class="show-more-less-button ms-4"
            @click="maxShown += 5"
            >Show more</v-btn
          >
          <v-btn
            density="compact"
            variant="plain"
            rounded
            color="secondary"
            v-else-if="maxShown > 5"
            class="show-more-less-button ms-4"
            @click="maxShown = 5"
            >Show less</v-btn
          >
        </div>
      </div>
    </v-expand-transition>
  </div>
</template>

<style scoped>
.albums-nav {
  display: flex;
  align-items: center;
  gap: 5px;
}

.albums-nav-item {
  flex-grow: 1;
}

.albums-nav-btn {
  transition: transform 0.3s ease-in-out;
}

.point-down {
  transform: rotate(180deg);
}

.expandable-list-branch {
  margin-left: 20px;
  padding-left: 5px;
  border-left: 1px solid rgba(var(--v-border-color), 0.12);
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.show-more-less-button {
  font-size: 11px;
  text-transform: none;
  justify-content: start;
  width: fit-content;
}
</style>
