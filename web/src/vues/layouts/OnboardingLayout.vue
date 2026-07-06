<script setup lang="ts">
withDefaults(
  defineProps<{
    text: string
    captionText?: boolean
    hideLogo?: boolean
  }>(),
  {
    captionText: false,
    hideLogo: false,
  },
)
</script>

<template>
  <div class="title-box">
    <div class="left-title" v-if="!hideLogo">
      <div class="big-image"></div>
    </div>
    <div class="right-title">
      <slot>
        <h1>Set Up <span>Ruurd Photos</span></h1>
      </slot>
      <p
        class="mt-2"
        :class="{
          'text-caption': captionText,
        }"
        v-text="text"
      />
    </div>
  </div>

  <v-divider class="mt-10 mb-5" />
</template>

<style scoped>
.title-box {
  display: flex;
  gap: 40px;
}

.big-image {
  background-image: url('img/app-no-bg-1024.png');
  width: 100px;
  height: 100px;
  background-size: 90%;
  background-position: center;
}

.right-title {
  color: rgb(var(--v-theme-on-surface-variant));
  height: 100px;
  display: flex;
  flex-direction: column;
  justify-content: center;
}

.title-box h1 {
  font-size: 35px;
  font-weight: 400;
  opacity: 0.9;
}

.title-box span {
  font-weight: 600;
}
</style>
