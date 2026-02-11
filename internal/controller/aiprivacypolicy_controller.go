/*
Copyright 2024.

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
*/

package controller

import (
	"context"
	"encoding/json"
	"fmt"

	corev1 "k8s.io/api/core/v1"
	"k8s.io/apimachinery/pkg/api/errors"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/apimachinery/pkg/runtime"
	ctrl "sigs.k8s.io/controller-runtime"
	"sigs.k8s.io/controller-runtime/pkg/client"
	"sigs.k8s.io/controller-runtime/pkg/log"

	privacyv1alpha1 "github.com/example/ai-privacy-shield/api/v1alpha1"
)

// AIPrivacyPolicyReconciler reconciles a AIPrivacyPolicy object
type AIPrivacyPolicyReconciler struct {
	client.Client
	Scheme *runtime.Scheme
}

//+kubebuilder:rbac:groups=privacy.example.com,resources=aiprivacypolicies,verbs=get;list;watch;create;update;patch;delete
//+kubebuilder:rbac:groups=privacy.example.com,resources=aiprivacypolicies/status,verbs=get;update;patch
//+kubebuilder:rbac:groups=privacy.example.com,resources=aiprivacypolicies/finalizers,verbs=update
//+kubebuilder:rbac:groups="",resources=configmaps,verbs=get;list;watch;create;update;patch;delete

// Reconcile is part of the main kubernetes reconciliation loop which aims to
// move the current state of the cluster closer to the desired state.
func (r *AIPrivacyPolicyReconciler) Reconcile(ctx context.Context, req ctrl.Request) (ctrl.Result, error) {
	_ = log.FromContext(ctx)

	// Fetch the AIPrivacyPolicy instance
	policy := &privacyv1alpha1.AIPrivacyPolicy{}
	err := r.Get(ctx, req.NamespacedName, policy)
	if err != nil {
		if errors.IsNotFound(err) {
			// Request object not found, could have been deleted after reconcile request.
			// Owned objects are automatically garbage collected. For additional cleanup logic use finalizers.
			// Return and don't requeue
			return ctrl.Result{}, nil
		}
		// Error reading the object - requeue the request.
		return ctrl.Result{}, err
	}

	// Define the ConfigMap name based on the policy name
	configMapName := fmt.Sprintf("privacy-policy-%s", policy.Name)

	// Serialize rules to JSON to be stored in ConfigMap
	rulesJson, err := json.Marshal(policy.Spec.Rules)
	if err != nil {
		return ctrl.Result{}, err
	}
	
	// Create or Update ConfigMap
	desiredConfigMap := &corev1.ConfigMap{
		ObjectMeta: metav1.ObjectMeta{
			Name:      configMapName,
			Namespace: policy.Namespace,
		},
		Data: map[string]string{
			"rules.json":  string(rulesJson),
			"destination": policy.Spec.Destination,
		},
	}

	// Set AIPrivacyPolicy instance as the owner and controller
	if err := ctrl.SetControllerReference(policy, desiredConfigMap, r.Scheme); err != nil {
		return ctrl.Result{}, err
	}

	foundConfigMap := &corev1.ConfigMap{}
	err = r.Get(ctx, client.ObjectKey{Name: configMapName, Namespace: policy.Namespace}, foundConfigMap)
	if err != nil && errors.IsNotFound(err) {
		err = r.Create(ctx, desiredConfigMap)
		if err != nil {
			return ctrl.Result{}, err
		}
	} else if err != nil {
		return ctrl.Result{}, err
	} else {
		// Update existing ConfigMap
		foundConfigMap.Data = desiredConfigMap.Data
		err = r.Update(ctx, foundConfigMap)
		if err != nil {
			return ctrl.Result{}, err
		}
	}

	// Update Status (Optional demo)
	policy.Status.Conditions = []metav1.Condition{
		{
			Type:               "Ready",
			Status:             metav1.ConditionTrue,
			Reason:             "ConfigMapSynced",
			Message:            fmt.Sprintf("ConfigMap %s synced successfully", configMapName),
			LastTransitionTime: metav1.Now(),
		},
	}
	if err := r.Status().Update(ctx, policy); err != nil {
		return ctrl.Result{}, err
	}

	return ctrl.Result{}, nil
}

// SetupWithManager sets up the controller with the Manager.
func (r *AIPrivacyPolicyReconciler) SetupWithManager(mgr ctrl.Manager) error {
	return ctrl.NewControllerManagedBy(mgr).
		For(&privacyv1alpha1.AIPrivacyPolicy{}).
		Owns(&corev1.ConfigMap{}).
		Complete(r)
}
