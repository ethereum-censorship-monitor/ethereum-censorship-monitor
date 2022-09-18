<template>
  <p v-if="data === null" class="text-center">loading...</p>
  <div v-else-if="data.length > 0">
    <h1 class="mt-12 mb-12 text-center text-4xl font-bold">
      Recent <span class="text-green-300">validators</span> who
      <span class="text-red-300">censored</span> transactions
    </h1>
    <Table :header="header" :data="data" :types="types" />
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
      header: ["Address", "Last censored block"],
      data: null,
      types: ["address", "block"],
    };
  },

  mounted() {
    this.fetchData();
  },

  methods: {
    async fetchData() {
      const url = new URL(
        "/v1/validators",
        import.meta.env.VITE_REST_API_ENDPOINT
      );
      const response = await fetch(url);
      const requestData = await response.json();
      let data = [];
      for (let i = 0; i < requestData.length; i++) {
        const requestRow = requestData[i];
        const row = [requestRow.validator, requestRow.last_censored_block];
        data.push(row);
      }
      this.data = data;
    },
  },
};
</script>
