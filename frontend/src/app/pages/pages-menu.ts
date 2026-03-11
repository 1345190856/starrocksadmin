import { NbMenuItem } from '@nebular/theme';

export const MENU_ITEMS: NbMenuItem[] = [
  {
    title: '集群列表',
    icon: 'list-outline',
    link: '/pages/starrocks/dashboard',
    home: true,
    data: { permission: 'menu:dashboard' },
  } as NbMenuItem & { data?: { permission: string } },
  {
    title: '集群概览',
    icon: 'activity-outline',
    link: '/pages/starrocks/overview',
    data: { permission: 'menu:overview' },
  } as NbMenuItem & { data?: { permission: string } },
  {
    title: '资源管理',
    icon: 'activity-outline',
    link: '/pages/resource/view',
    data: { permission: 'menu:resource' },
  } as NbMenuItem & { data?: { permission: string } },
  {
    title: '告警管理',
    icon: 'activity-outline',
    data: { permission: 'menu:alert' },
    children: [
      {
        title: '告警大盘',
        link: '/pages/alert/dashboard',
        data: { permission: 'menu:alert:dashboard' },
      } as NbMenuItem & { data?: { permission: string } },
      {
        title: '告警规则',
        link: '/pages/alert/rules',
        data: { permission: 'menu:alert:rules' },
      } as NbMenuItem & { data?: { permission: string } },
    ],
  } as NbMenuItem & { data?: { permission: string } },
  {
    title: '应用管理',
    icon: 'browser-outline',
    link: '/pages/application',
    data: { permission: 'menu:application' },
  } as NbMenuItem & { data?: { permission: string } },
  {
    title: '节点管理',
    icon: 'hard-drive-outline',
    data: { permission: 'menu:nodes' },
    children: [
      {
        title: 'Frontend 节点',
        link: '/pages/starrocks/frontends',
        data: { permission: 'menu:nodes:frontends' },
      } as NbMenuItem & { data?: { permission: string } },
      {
        title: 'Backend 节点',
        link: '/pages/starrocks/backends',
        data: { permission: 'menu:nodes:backends' },
      } as NbMenuItem & { data?: { permission: string } },
    ],
  } as NbMenuItem & { data?: { permission: string } },
  {
    title: '查询管理',
    icon: 'code-outline',
    data: { permission: 'menu:queries' },
    children: [
      {
        title: '实时查询',
        link: '/pages/starrocks/queries/execution',
        data: { permission: 'menu:queries:execution' },
      } as NbMenuItem & { data?: { permission: string } },
      {
        title: 'Profiles',
        link: '/pages/starrocks/queries/profiles',
        data: { permission: 'menu:queries:profiles' },
      } as NbMenuItem & { data?: { permission: string } },
      {
        title: '审计日志',
        link: '/pages/starrocks/queries/audit-logs',
        data: { permission: 'menu:queries:audit-logs' },
      } as NbMenuItem & { data?: { permission: string } },
    ],
  } as NbMenuItem & { data?: { permission: string } },
  {
    title: '物化视图',
    icon: 'cube-outline',
    link: '/pages/starrocks/materialized-views',
    data: { permission: 'menu:materialized-views' },
  } as NbMenuItem & { data?: { permission: string } },
  {
    title: '功能卡片',
    icon: 'grid-outline',
    link: '/pages/starrocks/system',
    data: { permission: 'menu:system-functions' },
  } as NbMenuItem & { data?: { permission: string } },
  {
    title: '会话管理',
    icon: 'person-outline',
    link: '/pages/starrocks/sessions',
    data: { permission: 'menu:sessions' },
  } as NbMenuItem & { data?: { permission: string } },
  {
    title: '变量管理',
    icon: 'settings-2-outline',
    link: '/pages/starrocks/variables',
    data: { permission: 'menu:variables' },
  } as NbMenuItem & { data?: { permission: string } },
  {
    title: '数据源',
    icon: 'layers-outline',
    link: '/pages/resource/datasource',
    data: { permission: 'menu:datasource' },
  } as NbMenuItem & { data?: { permission: string } },
  {
    title: '人力资源',
    icon: 'people-outline',
    link: '/pages/headcount',
    data: { permission: 'menu:headcount' },
  } as NbMenuItem & { data?: { permission: string } },
  {
    title: '资产管理',
    icon: 'cube-outline',
    link: '/pages/asset-inventory',
    data: { permission: 'menu:asset_inventory' },
  } as NbMenuItem & { data?: { permission: string } },
  {
    title: '数据同步',
    icon: 'shuffle-2-outline',
    link: '/pages/data-sync',
    data: { permission: 'menu:data_sync' },
  } as NbMenuItem & { data?: { permission: string } },
  {
    title: '值班卡',
    icon: 'calendar-outline',
    link: '/pages/duty/personnel',
    data: { permission: 'menu:duty' },
  } as NbMenuItem & { data?: { permission: string } },

  {
    title: '系统管理',
    icon: 'settings-outline',
    data: { permission: 'menu:system' }, // Parent menu permission
    children: [
      {
        title: '用户管理',
        link: '/pages/system/users',
        data: { permission: 'menu:system:users' },
      } as NbMenuItem & { data?: { permission: string } },
      {
        title: '角色管理',
        link: '/pages/system/roles',
        data: { permission: 'menu:system:roles' },
      } as NbMenuItem & { data?: { permission: string } },
      {
        title: '组织管理',
        link: '/pages/system/organizations',
        data: { permission: 'menu:system:organizations' },
      } as NbMenuItem & { data?: { permission: string } },
    ],
  } as NbMenuItem & { data?: { permission: string } },
];
