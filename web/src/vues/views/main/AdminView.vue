<script setup lang="ts">
import MainLayoutContainer from '@/vues/components/MainLayoutContainer.vue'
import UserAdmin from '@/vues/components/admin/UserAdmin.vue'
import JobsAdmin from '@/vues/components/admin/JobsAdmin.vue'
</script>

<template>
  <main-layout-container class="settings">
    <div class="settings-content">
      <h1 class="settings-title">Administration</h1>

      <div class="divider-flex">
        <v-divider />
        <span>Users</span>
        <v-divider />
      </div>
      <user-admin />

      <div class="divider-flex">
        <v-divider />
        <span>Jobs</span>
        <v-divider />
      </div>
      <jobs-admin />

      <!-- Section: Placeholder Other Settings -->
      <section class="placeholder-section">
        <h3 class="placeholder-title">Other Admin stuff Placeholder</h3>
        <p class="placeholder-desc">Hello world</p>
      </section>
    </div>
  </main-layout-container>
</template>

<style scoped>
.settings {
  overflow-y: auto;
}

.settings-content {
  max-width: 1400px;
  margin: 0 auto;
  padding: 32px 24px;
}

.settings-title {
  font-size: 2.125rem;
  font-weight: 700;
  margin-bottom: 32px;
  color: rgb(var(--v-theme-on-surface));
}

.divider-flex {
  display: flex;
  gap: 30px;
  margin-top: 25px;
  margin-bottom: 25px;
  margin-left: 50px;
  margin-right: 50px;
  align-items: center;
}

.divider-flex span {
  font-size: 25px;
  white-space: nowrap;
  color: rgba(var(--v-theme-on-surface-variant), 0.8);
}

.placeholder-section {
  margin-top: 32px;
  border-top: 1px solid rgba(var(--v-border-color), var(--v-border-opacity));
  padding-top: 24px;
}

.placeholder-title {
  font-size: 1.25rem;
  font-weight: 500;
  color: rgb(var(--v-theme-on-surface-variant));
  margin-bottom: 8px;
}

.placeholder-desc {
  font-size: 0.875rem;
  color: rgb(var(--v-theme-on-surface-variant));
}
</style>
