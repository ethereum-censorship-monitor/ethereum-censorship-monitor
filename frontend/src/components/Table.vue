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
        <td v-for="cell in row" :key="cell" class="h-12 text-xl font-mono">
          {{ cell }}
        </td>
      </tr>
    </tbody>
  </table>
</template>

<script>
export default {
  name: "Table",
  props: ["header", "data"],
};
</script>
