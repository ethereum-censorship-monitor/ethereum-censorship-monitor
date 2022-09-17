<template>
  <h1 class="mt-12 mb-12 text-center text-4xl font-bold">
    Recent <span class="text-green-300">blocks</span> that
    <span class="text-red-300">censored</span> transactions
  </h1>
  <div class="mx-auto">
    <Table :header="header" :data="data" />
  </div>
</template>

<script>
import Table from "../components/Table.vue";

export default {
  name: "Blocks",
  components: { Table },
  data() {
    return {
      header: ["Number", "Validator", "Hash"],
      data: [
        [
          "15552401",
          "329690",
          "0x227a39912cfac898cf2a03671b5395150f62daf676ea7228807db2ebc7978b36",
        ],
        [
          "15552401",
          "329690",
          "0x227a39912cfac898cf2a03671b5395150f62daf676ea7228807db2ebc7978b36",
        ],
        [
          "15552401",
          "329690",
          "0x227a39912cfac898cf2a03671b5395150f62daf676ea7228807db2ebc7978b36",
        ],
      ],
    };
  },
};
</script>
