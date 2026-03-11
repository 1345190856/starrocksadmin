import { RouterModule, Routes } from '@angular/router';
import { NgModule } from '@angular/core';

import { PagesComponent } from './pages.component';

const routes: Routes = [{
  path: '',
  component: PagesComponent,
  children: [
    {
      path: 'starrocks',
      loadChildren: () => import(/* webpackChunkName: "starrocks" */ './starrocks/starrocks.module')
        .then(m => m.StarRocksModule),
    },
    {
      path: 'user-settings',
      loadChildren: () => import('./user-settings/user-settings.module')
        .then(m => m.UserSettingsModule),
    },
    {
      path: 'system',
      loadChildren: () => import('./system/system.module')
        .then(m => m.SystemModule),
    },
    {
      path: 'duty',
      loadChildren: () => import('./duty/duty.module')
        .then(m => m.DutyModule),
    },
    {
      path: 'resource',
      loadChildren: () => import('./resource/resource.module')
        .then(m => m.ResourceModule),
    },
    {
      path: 'headcount',
      loadChildren: () => import('./headcount/headcount.module')
        .then(m => m.HeadcountModule),
    },
    {
      path: 'alert',
      loadChildren: () => import('./alert/alert.module')
        .then(m => m.AlertModule),
    },
    {
      path: 'application',
      loadChildren: () => import('./application/application.module')
        .then(m => m.ApplicationModule),
    },
    {
      path: 'asset-inventory',
      loadChildren: () => import('./asset-inventory/asset-inventory.module')
        .then(m => m.AssetInventoryModule),
    },
    {
      path: 'data-sync',
      loadChildren: () => import('./data-sync/data-sync.module')
        .then(m => m.DataSyncModule),
    },
    {
      path: '',
      redirectTo: 'starrocks',
      pathMatch: 'full',
    },
  ],
}];

@NgModule({
  imports: [RouterModule.forChild(routes)],
  exports: [RouterModule],
})
export class PagesRoutingModule {
}
