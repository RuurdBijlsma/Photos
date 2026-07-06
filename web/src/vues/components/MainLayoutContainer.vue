<script setup lang="ts">
withDefaults(
  defineProps<{
    fitToContent?: boolean
    hideDropShadow?: boolean
    ignoreScrollBar?: boolean
  }>(),
  {
    fitToContent: false,
    hideDropShadow: false,
    ignoreScrollBar: false,
  },
)
</script>

<template>
  <div
    class="outer-container"
    :class="{
      'fit-content': fitToContent,
      'hide-drop-shadow': hideDropShadow,
      'ignore-scrollbar': ignoreScrollBar,
    }"
  >
    <div class="inner-container">
      <slot></slot>
    </div>
  </div>
</template>

<style scoped>
.outer-container {
  background: linear-gradient(
    0deg,
    rgba(var(--v-theme-background), 0.8) 0%,
    rgba(var(--v-theme-background), 0.9) 100%
  );
  flex-grow: 1;
  border-top-left-radius: 60px;
  border-top-right-radius: 60px;
  overflow: hidden;
  box-shadow:
    0 10px 200px 0 rgba(var(--v-theme-primary-lighten-1), 0.1),
    0 10px 20px 0 rgba(0, 0, 0, 0.1);
  max-width: calc(100% - 50px);
  width: 100%;
  height: 100%;
}

.outer-container.ignore-scrollbar {
  max-width: 100%;
}

.hide-drop-shadow.outer-container {
  box-shadow: none !important;
}

.outer-container.fit-content {
  flex: none;
  height: fit-content;
  min-height: 0;
  display: flex;
  flex-direction: column;
  align-self: flex-start;
  border-bottom-left-radius: 60px;
  border-bottom-right-radius: 60px;
  margin-bottom: 40px;
}

.v-theme--light .outer-container {
  backdrop-filter: brightness(150%) saturate(250%) blur(50px) contrast(100%) !important;
}

.v-theme--dark .outer-container {
  backdrop-filter: brightness(50%) saturate(250%) blur(50px) contrast(100%) !important;
}

.inner-container {
  height: calc(100% - 10px);
  width: calc(100% - 20px);
  margin: 10px 10px 0;
  border-radius: 50px 50px 0 0;
  overflow: hidden;
  overflow-y: auto;

  -ms-overflow-style: none;
  scrollbar-width: none;
}

.fit-content .inner-container {
  flex: none;
  height: auto;
  border-radius: 50px;
  overflow-y: visible;
  margin-bottom: 10px;
}

.inner-container::-webkit-scrollbar {
  display: none;
}
</style>
