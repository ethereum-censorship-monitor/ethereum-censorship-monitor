<template>
  <div v-if="data.length > 0">
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

export default {
  name: "Transactions",
  components: { Table },
  data() {
    return {
      header: ["Timestamp", "Hash"],
      data: [
        [
          1663402092,
          "0xb1dc7b027e45efaab5d4f62004f3ba128480454a05230719043b038c6aac1daa",
        ],
        [
          1663402093,
          "0xb1dc7b027e45efaab5d4f62004f3ba128480454a05230719043b038c6aac1daa",
        ],
        [
          1663402094,
          "0xb1dc7b027e45efaab5d4f62004f3ba128480454a05230719043b038c6aac1daa",
        ],
      ],
    };
  },
};
</script>
