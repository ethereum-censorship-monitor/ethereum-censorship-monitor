<template>
  <div class="grid grid-cols-3 gap-28 mt-12">
    <div class="text-4xl font-bold p-4 text-center">
      In the past 24h,
      <span v-if="numValidators > 0">
        at least
        <span class="text-red-300">
          {{ numValidators }} validators censored
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
      <span v-if="numBlocks > 0">
        at least
        <span class="text-red-300">{{ numBlocks }} blocks censored</span>
        transactions.
      </span>
      <span v-else class="text-green-300">
        no blocks <span class="text-slate-100">appeared to have</span>
        censored transactions.
      </span>
    </div>
    <div class="text-4xl font-bold p-4 text-center">
      In the past 24h,
      <span v-if="numTransactions > 0">
        at least
        <span class="text-red-300"
          >{{ numTransactions }} transactions
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
</template>

<script>
export default {
  name: "Overview",
  data() {
    return {
      numValidators: 0,
      numBlocks: 0,
      numTransactions: 0,
    };
  },
};
</script>
