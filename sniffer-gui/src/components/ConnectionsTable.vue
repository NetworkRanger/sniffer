<script setup lang="ts">
import type {Connection} from "../types/connection";
import {filesize} from "filesize";

defineProps<{ connections: Connection[] }>();

const bytesize = (bytes: number) =>
    filesize(bytes, {base: 2, standard: "jedec", round: 1}).replace(" ", "");

const speedsize = (bytes: number) => bytesize(bytes) + "/s";

const formatTime = (ts: number) => ts ? new Date(ts * 1000).toLocaleString() : '-';
</script>

<template>
  <el-table
      :data="connections"
      style="width: 100%; flex: 1;"
      size="small"
      stripe>
    <el-table-column label="进程名称" width="90">
      <template #default="{ row }">{{ row.process_connection?.process_name || '-' }}</template>
    </el-table-column>
    <el-table-column label="进程ID" width="65" align="right">
      <template #default="{ row }">{{ row.process_connection?.pid || '-' }}</template>
    </el-table-column>
    <el-table-column label="协议" width="55">
      <template #default="{ row }">{{ row.protocol }}</template>
    </el-table-column>
    <el-table-column label="应用协议" width="75">
      <template #default="{ row }">{{ row.packet_connection?.protocol || '-' }}</template>
    </el-table-column>
    <el-table-column label="总量" width="85" align="right">
      <template #default="{ row }">{{ bytesize(row.bytes_sent + row.bytes_recv) }}</template>
    </el-table-column>
    <el-table-column label="上传" width="85" align="right">
      <template #default="{ row }">{{ speedsize(row.upload_speed) }}</template>
    </el-table-column>
    <el-table-column label="下载" width="85" align="right">
      <template #default="{ row }">{{ speedsize(row.download_speed) }}</template>
    </el-table-column>
    <el-table-column label="本地地址" width="115">
      <template #default="{ row }">{{ row.local_addr }}</template>
    </el-table-column>
    <el-table-column label="本地端口" width="75" align="right">
      <template #default="{ row }">{{ row.local_port }}</template>
    </el-table-column>
    <el-table-column label="远程地址" min-width="140">
      <template #default="{ row }">{{ row.domain || row.remote_addr }}</template>
    </el-table-column>
    <el-table-column label="远程端口" width="75" align="right">
      <template #default="{ row }">{{ row.remote_port }}</template>
    </el-table-column>
    <el-table-column label="状态" width="95">
      <template #default="{ row }">{{ row.status }}</template>
    </el-table-column>
    <el-table-column label="域名" width="140">
      <template #default="{ row }">{{ row.domain || '-' }}</template>
    </el-table-column>
    <el-table-column label="路径" width="80">
      <template #default="{ row }">{{ row.path || '-' }}</template>
    </el-table-column>
    <el-table-column label="活跃时间" width="150">
      <template #default="{ row }">{{ formatTime(row.last_active) }}</template>
    </el-table-column>
  </el-table>
</template>
