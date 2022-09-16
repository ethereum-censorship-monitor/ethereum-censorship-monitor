<template>
  <div class="min-h-screen bg-slate-800 text-slate-100 font-sans">
    <NavBar />
    <router-view></router-view>
  </div>
</template>

<script>
import NavBar from "./components/NavBar.vue";

export default {
  name: "App",
  components: { NavBar },
};
</script>
