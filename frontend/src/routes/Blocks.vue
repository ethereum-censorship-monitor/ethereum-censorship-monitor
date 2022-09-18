<template>
  <p v-if="data === null" class="text-center">loading...</p>
  <div v-else-if="data.length > 0">
    <h1 class="mb-12 text-center text-4xl font-bold">
      Recent <span class="text-green-300">blocks</span> that
      <span class="text-red-300">censored</span> transactions
    </h1>
    <Table :header="header" :data="data" :types="types" />
  </div>
  <p v-else class="text-center text-4xl font-bold text-green-300">
    No censoring blocks detected
  </p>
</template>

<script>
import Table from "../components/Table.vue";

export default {
  name: "Blocks",
  components: { Table },
  mounted() {
    this.fetchData();
  },
  data() {
    return {
      header: ["Number", "Validator", "Hash"],
      data: null,
      types: ["", "address", "block"],
    };
  },

  methods: {
    async fetchData() {
      const url = new URL("/v1/blocks", import.meta.env.VITE_REST_API_ENDPOINT);
      const response = await fetch(url);
      const requestData = await response.json();
      let data = [];
      for (let i = 0; i < requestData.length; i++) {
        const requestRow = requestData[i];
        const row = [
          requestRow.block_number,
          requestRow.validator,
          requestRow.hash,
        ];
        data.push(row);
      }
      this.data = data;
    },
  },
};
</script>
