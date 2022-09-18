<template>
  <table class="mx-auto table-auto border-separate border-spacing-x-8">
    <thead>
      <tr>
        <th v-for="title in header" :key="title" class="text-2xl text-left">
          {{ title }}
        </th>
      </tr>
    </thead>
    <tbody>
      <tr v-for="row in data" :key="row">
        <td v-for="(cell, i) in row" :key="cell" class="h-12 text-xl font-mono">
          <a
            v-if="types && types[i] === 'address'"
            :href="'https://etherscan.io/address/' + cell"
            class="underline decoration-dashed"
            >{{ cell }}</a
          >
          <a
            v-else-if="types && types[i] === 'block'"
            :href="'https://etherscan.io/block/' + cell"
            class="underline decoration-dashed"
            >{{ cell }}</a
          >
          <a
            v-else-if="types && types[i] === 'tx'"
            :href="'https://etherscan.io/tx/' + cell"
            class="underline decoration-dashed"
            >{{ cell }}</a
          >
          <span v-else>{{ cell }}</span>
        </td>
      </tr>
    </tbody>
  </table>
</template>

<script>
export default {
  name: "Table",
  props: ["header", "data", "types"],
};
</script>
