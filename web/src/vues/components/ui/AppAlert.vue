<script setup lang="ts">
defineProps<{
  icon: string
}>()

const errorMessage = defineModel({
  type: String,
  default: '',
  required: false,
})
</script>

<template>
  <v-alert v-if="errorMessage" variant="flat" color="error" class="server-alert">
    <v-icon color="on-error" :icon="icon" />
    <p class="text-caption on-error-text">
      {{ errorMessage }}
    </p>
    <v-spacer></v-spacer>
    <v-btn @click="errorMessage = ''" icon="mdi-close" variant="plain" density="compact" />
  </v-alert>
</template>

<style scoped>
.server-alert {
  border-radius: 15px;
}

.server-alert:deep(.v-alert__content) {
  display: flex;
  flex-direction: row;
  align-items: center;
  gap: 20px;
  padding-left: 10px;
}

.on-error-text {
  color: rgb(var(--v-theme-on-error));
}
</style>
