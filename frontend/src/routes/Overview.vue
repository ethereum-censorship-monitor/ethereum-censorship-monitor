<template>
  <div>
    <p v-if="stats === null" class="text-center">loading...</p>
    <div v-else class="grid grid-cols-3 gap-28">
      <div class="text-4xl font-bold p-4 text-center">
        In the past 24h,
        <span v-if="stats.numValidators > 0">
          at least
          <span class="text-red-300">
            {{ stats.numValidators }} validators censored
          </span>
          transactions.
        </span>
        <span v-else class="text-green-300">
          no validators <span class="text-slate-100">appeared to have</span>
          censored transactions.
        </span>
      </div>
      <div class="text-4xl font-bold p-4 text-center">
        In the past 24h,
        <span v-if="stats.numBlocks > 0">
          at least
          <span class="text-red-300"
            >{{ stats.numBlocks }} blocks censored</span
          >
          transactions.
        </span>
        <span v-else class="text-green-300">
          no blocks <span class="text-slate-100">appeared to have</span>
          censored transactions.
        </span>
      </div>
      <div class="text-4xl font-bold p-4 text-center">
        In the past 24h,
        <span v-if="stats.numTransactions > 0">
          at least
          <span class="text-red-300"
            >{{ stats.numTransactions }} transactions
            <span class="text-slate-100">were</span> censored</span
          >
        </span>
        <span v-else class="text-green-300">
          no transactions
          <span class="text-slate-100">appeared to have been</span>
          censored.
        </span>
      </div>
    </div>
  </div>
</template>

<script>
export default {
  name: "Overview",
  data() {
    return {
      stats: null,
    };
  },

  mounted() {
    this.fetchData();
  },

  methods: {
    async fetchData() {
      const url = new URL("/v1/stats", import.meta.env.VITE_REST_API_ENDPOINT);
      const response = await fetch(url);
      const requestData = await response.json();
      this.stats = {
        numTransactions: requestData.num_transactions,
        numBlocks: requestData.num_blocks,
        numValidators: requestData.num_validators,
      };
    },
  },
};
</script>
