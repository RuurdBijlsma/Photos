<script setup lang="ts"></script>

<template>
  <v-main class="main">
    <div class="container">
      <slot />
    </div>
  </v-main>
</template>

<style scoped>
.main {
  position: fixed;
  width: 100%;
  height: 100%;
  overflow-y: auto;
  display: flex;
  justify-content: center;
  padding: 20px;
  background-color: rgb(var(--v-theme-surface-container));
}

.container {
  margin: auto 0;
  flex-grow: 0;
  border-radius: 40px;
  overflow: hidden;
  box-shadow: 0 10px 20px 0 rgba(0, 0, 0, 0.1);
  padding: 40px;
  max-width: 800px;
  display: flex;
  flex-direction: column;
  background: linear-gradient(
    0deg,
    rgba(var(--v-theme-background), 0.7) 0%,
    rgba(var(--v-theme-background), 1) 100%
  );
}

.container:deep(.v-sheet) {
  background-color: transparent !important;
  box-shadow: none !important;
}

.container:deep(.v-stepper-header) {
  box-shadow: none !important;
}
</style>
