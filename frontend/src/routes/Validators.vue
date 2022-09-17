<template>
  <div v-if="data.length > 0">
    <h1 class="mt-12 mb-12 text-center text-4xl font-bold">
      Recent <span class="text-green-300">validators</span> who
      <span class="text-red-300">censored</span> transactions
    </h1>
    <Table :header="header" :data="data" />
  </div>
  <p v-else class="mt-12 text-center text-4xl font-bold text-green-300">
    No censoring validators detected
  </p>
</template>

<script>
import Table from "../components/Table.vue";

export default {
  name: "Validators",
  components: { Table },
  data() {
    return {
      header: ["Index", "Censored transactions"],
      data: [
        [33681, 12],
        [123, 3],
        [39189, 1],
      ],
    };
  },
};
</script>
