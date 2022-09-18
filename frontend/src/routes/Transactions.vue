<template>
  <p v-if="data === null" class="text-center">loading...</p>
  <div v-else-if="data.length > 0">
    <h1 class="mt-12 mb-12 text-center text-4xl font-bold">
      Recently <span class="text-red-300">censored </span>
      <span class="text-green-300">transactions</span>
    </h1>
    <Table :header="header" :data="data" />
  </div>
  <p v-else class="mt-12 text-center text-4xl font-bold text-green-300">
    No censored transactions detected
  </p>
</template>

<script>
import Table from "../components/Table.vue";

function formatTimestamp(timestamp) {
  const date = new Date(timestamp * 1000);
  return date.toUTCString();
}

export default {
  name: "Transactions",
  components: { Table },
  data() {
    return {
      header: ["Timestamp", "Hash"],
      data: null,
    };
  },

  mounted() {
    this.fetchData();
  },

  methods: {
    async fetchData() {
      const url = new URL(
        "/v1/transactions",
        import.meta.env.VITE_REST_API_ENDPOINT
      );
      const response = await fetch(url);
      const requestData = await response.json();
      let data = [];
      for (let i = 0; i < requestData.length; i++) {
        const requestRow = requestData[i];
        const row = [formatTimestamp(requestRow.first_seen), requestRow.hash];
        data.push(row);
      }
      this.data = data;
    },
  },
};
</script>
