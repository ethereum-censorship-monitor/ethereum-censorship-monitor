<template>
  <nav class="bg-slate-900">
    <div class="container mx-auto h-20 flex flex-col justify-center">
      <div class="h-full mx-8 flex flex-row justify-between">
        <div class="flex flex-col justify-center">
          <p class="text-2xl text-green-300 font-bold">
            Ethereum <span class="text-red-300">Censorship</span> Monitor
          </p>
        </div>
        <ul class="flex flex-row h-full justify-end gap-4 text-xl">
          <router-link
            tag="li"
            v-for="route in $router.getRoutes()"
            :key="route.path"
            class="border-b-2 border-transparent hover:border-current flex flex-col justify-center"
            :class="{ 'border-current': this.$route.name === route.name }"
            :to="{ name: route.name }"
          >
            <p>{{ route.meta.title }}</p>
          </router-link>
        </ul>
      </div>
    </div>
  </nav>
</template>

<script>
export default {
  name: "NavBar",
};
</script>
